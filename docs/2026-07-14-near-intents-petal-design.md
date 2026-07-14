# NEAR Intents 1Click Petal Design

**Status:** implementation specification
**Date:** 2026-07-14
**Target:** `bloom-petal-near`
**Bloom compatibility:** local Petal app v2, `bloom:route@0.1.0`

## 1. Decision

Build a local Bloom Petal app mounted at `/apps/near-intents` that executes
NEAR Intents swaps through the 1Click Swap API.

Version 1 supports:

- a Bloom-managed EVM wallet as the origin wallet;
- an origin asset on an EVM chain configured in Bloom and recognized by this
  Petal;
- any destination asset and recipient accepted by 1Click;
- `EXACT_INPUT` and `EXACT_OUTPUT` swaps;
- native EVM and ERC-20 deposits;
- authenticated 1Click API access using a persistently stored partner JWT;
- signed-quote verification before a deposit transaction can be staged;
- Bloom's generic EVM outbox, policy engine, plan, simulation, broadcast
  controls, Sealed Approval, and audit trail; and
- persisted deposit submission, status, refund, and receipt information.

The Petal must not delegate to a native NEAR Intents handler in Bloom. No such
handler is required. The implementation uses only generic v2 Petal host
interfaces.

## 2. Motivation

Bloom already provides the boundaries needed for an agent-facing swap client:

- a filesystem route model;
- content-addressed WASM component execution;
- default-deny HTTPS access;
- private per-package state and secret namespaces;
- mediated EVM reads;
- generic EVM transaction staging and inspection; and
- review and approval before signing or broadcast.

The recommended NEAR Intents distribution-channel flow is likewise bounded:
request a quote, deposit to the returned origin-chain address, optionally
submit the deposit transaction hash, and poll status. A Petal can own the
protocol-specific interpretation while Bloom retains key custody and generic
transaction enforcement.

## 3. Goals and non-goals

### 3.1 Goals

1. Make a cross-chain swap inspectable and operable as files.
2. Bind the staged EVM deposit exactly to a verified, persisted 1Click quote.
3. Never expose the JWT, private store contents, signatures, or authorization
   headers through public VFS reads, errors, logs, or audit data.
4. Make every externally significant transition idempotent and recoverable
   after process restart.
5. Preserve the authoritative Bloom outbox plan rather than inventing a
   parallel transaction-signing path.
6. Fail closed on expired quotes, unsupported origin chains, invalid token
   metadata, invalid quote signatures, and ambiguous prior side effects.

### 3.2 Non-goals for version 1

- Native NEAR wallet custody, NEP-413 signing, or NEAR transaction submission.
- Solana, Bitcoin, TON, XRP, or other non-EVM origins.
- Direct Message Bus or `intents.near` verifier-contract integration.
- `INTENTS` or `CONFIDENTIAL_INTENTS` as the deposit type.
- `FLEX_INPUT`, `ANY_INPUT`, app fees, rebates, insurance, or confidential
  swaps.
- User authentication/session endpoints, account balances, or account history.
- Automated background jobs. Status advances when the user or agent writes to
  a refresh route.
- A top-level `/near-intents` mount. Local Petal apps live under `/apps`.
- Encrypted secret storage. Bloom's current secret namespace is private and
  mode-restricted but not an encrypted credential vault.

## 4. Upstream protocol contract

The implementation targets the OpenAPI document currently published at:

`https://1click.chaindefuser.com/docs/v0/openapi.yaml`

The production API origin is:

`https://1click.chaindefuser.com`

Required endpoints are:

| Method | Path | Purpose | Authentication |
|---|---|---|---|
| `GET` | `/v0/tokens` | Discover supported assets | None required |
| `POST` | `/v0/quote` | Request an executable quote | Partner JWT |
| `POST` | `/v0/deposit/submit` | Notify 1Click of the mined deposit transaction | Partner JWT |
| `GET` | `/v0/status` | Read swap execution status | Partner JWT |

For authenticated requests the Petal sends:

```http
Authorization: Bearer <jwt>
```

Version 1 requires the JWT for every endpoint except `/v0/tokens`. It does not
silently fall back to an unauthenticated, fee-bearing quote.

The status request must include `depositAddress`. If the quote includes
`depositMemo`, the request must also include `depositMemo`.

The submit request is:

```json
{
  "txHash": "0x...",
  "depositAddress": "0x..."
}
```

If the executable quote contains a memo, include it as `memo`. An EVM-origin
quote is expected not to require a memo; version 1 rejects an EVM executable
quote with a non-empty `depositMemo` before staging funds.

### 4.1 Supported request subset

The Petal sends the following quote fields:

```json
{
  "dry": false,
  "depositMode": "SIMPLE",
  "swapType": "EXACT_INPUT",
  "slippageTolerance": 100,
  "originAsset": "nep141:arb-0x....omft.near",
  "depositType": "ORIGIN_CHAIN",
  "destinationAsset": "nep141:sol-....omft.near",
  "amount": "1000000",
  "refundTo": "0x...",
  "refundType": "ORIGIN_CHAIN",
  "recipient": "...",
  "recipientType": "DESTINATION_CHAIN",
  "deadline": "...",
  "quoteWaitingTimeMs": 3000
}
```

The Petal omits `appFees`, `rebates`, `insured`, `confidentiality`,
`customRecipientMsg`, virtual-chain fields, and all account/session fields.
The live API may echo an omitted `insured` request field as `false`. The Petal
accepts absent or `false` because `insured` is excluded from the official SDK's
signed projection, but rejects `true` as unsupported execution metadata.
The unknown-field policy in section 4.3 controls schema evolution; retaining
raw response evidence does not make an unknown field acceptable for execution.

### 4.2 Status values

The Petal recognizes these upstream states:

- `PENDING_DEPOSIT`
- `KNOWN_DEPOSIT_TX`
- `INCOMPLETE_DEPOSIT`
- `PROCESSING`
- `SUCCESS`
- `REFUNDED`
- `FAILED`

An unknown upstream status is preserved but treated as non-terminal and
`upstream_unknown`; it must never be interpreted as success.

### 4.3 Unknown-field policy

Parse and retain bounded raw response bytes for diagnostics and dispute
evidence, but apply different typed-decoding rules by security role:

- `QuoteResponse`, its top-level object, `quoteRequest`, and `quote` are a
  security-critical execution envelope. Their typed DTOs enumerate every field
  in the pinned OpenAPI schema and reject unknown fields. Known but unsupported
  optional fields must be absent, null, false, empty, or their documented
  default as explicitly validated by the workflow; a non-default value is
  fatal unless this specification supports it.
- The `quoteResponse` embedded in `/v0/status` is decoded with the same strict
  DTOs and rules before it is correlated with the persisted quote.
- `/v0/tokens`, sanitized `swapDetails`, submit responses, and error responses
  may tolerate unknown fields because they do not define the already prepared
  deposit transaction. Public projections still expose only allowlisted known
  fields.

Thus a new unknown field anywhere in an executable quote envelope fails closed
until a reviewed Petal release classifies it. The raw bytes may still be stored
alongside the rejection. Tests must prove that an unknown quote-envelope field
is fatal while an unknown token or `swapDetails` field is ignored and retained
only as bounded evidence.

## 5. Package layout

Use the current Polymarket v2 Petal as the structural reference, while keeping
all NEAR Intents behavior local to this repository:

```text
bloom-petal-near/
├── AGENTS.md
├── README.md
├── petal.toml
├── docs/
│   └── 2026-07-14-near-intents-petal-design.md
├── petal/
│   ├── Cargo.toml
│   ├── src/lib.rs
│   └── wit/route.wit
├── route/
│   ├── Cargo.toml
│   ├── files/
│   └── src/
├── xtask/
│   ├── Cargo.toml
│   └── src/main.rs
├── scripts/
│   ├── build.sh
│   └── validate.sh
└── app/near-intents/          # generated; not committed
```

`petal/` is a generic local framework crate. It may initially be derived from
the Polymarket v2 Petal framework at the Bloom revision used for development,
but it must not contain NEAR Intents semantics. Protocol DTOs, quote
verification, state transitions, asset resolution, and transaction preparation
belong in focused modules under `route/src/`.

The derived framework must not copy the Polymarket signing surface unchanged.
Remove `import bloom:sign/signing@0.2.0` from `petal/wit/route.wit`, remove the
signing SDK wrappers and types, and confirm no generated route component imports
either version of `bloom:sign/signing`. Bloom install validation rejects a sign
import when `bloom:sign` and `[sign].allowed_intents` are absent; this Petal
deliberately declares neither.

Each route file builds as a `bloom:route@0.1.0` WASM component. Generated WASM,
`target/`, and package artifacts are ignored.

## 6. Manifest

The initial `petal.toml` must be equivalent to:

```toml
schema = "bloom.petal.local-app.v2"
name = "near-intents"

[source]
kind = "github"
repository = "bloom-directory/bloom-petal-near"

[build]
command = "scripts/build.sh"
outputs = ["app/near-intents"]

[consent]
summary = "Request verified NEAR Intents 1Click quotes, stage EVM deposits through Bloom, and track cross-chain swap settlement."

[caps]
allowed = [
  "bloom:http",
  "bloom:store",
  "bloom:tx.outbox",
  "bloom:chain",
  "bloom:vfs.read",
]

[[net.allow]]
binding = "oneclick"
host = "1click.chaindefuser.com"
methods = ["get"]
paths = ["/v0/tokens", "/v0/status"]

[[net.allow]]
binding = "oneclick"
host = "1click.chaindefuser.com"
methods = ["post"]
paths = ["/v0/quote", "/v0/deposit/submit"]

[store]
namespaces = ["state"]
secret_namespaces = ["secrets"]
```

The Petal does not need `bloom:sign`: 1Click execution is initiated by an EVM
deposit staged through `bloom:tx.outbox`. Bloom's transaction engine owns the
signing path.

The `oneclick` endpoint binding permits an operator-controlled HTTPS origin for
tests or an approved deployment, but does not widen methods or paths. Quote
signature verification remains pinned to the production manager key, so a
non-production endpoint cannot produce executable quotes unless a future,
explicitly reviewed design adds a separate trust-root configuration.

## 7. Persistent settings and secrets

### 7.1 API-key route

Expose:

```text
/apps/near-intents/settings/api-key
```

Writes accept either a raw non-whitespace JWT or:

```json
{"jwt":"..."}
```

Validation:

- UTF-8 only;
- after trimming, length `1..=8192`;
- no whitespace inside the token;
- reject control characters;
- never include the rejected value in an error.

The value is stored in the `secrets` namespace under
`credentials/partner-jwt`. A successful write replaces the previous token
atomically.

Reads must never return the credential. They return only:

```json
{
  "configured": true,
  "storage": "persistent_private_store",
  "encrypted_at_rest": false
}
```

The credential persists across daemon restarts and machine reboots. It is
stored under the installed package's private storage root with restrictive
filesystem permissions. Because Bloom currently keys this storage by package
content hash, a newly installed package version does not automatically inherit
the prior version's credential. Version 1 documents re-provisioning after an
upgrade; implicit secret migration is forbidden.

### 7.2 Secret handling rules

1. The raw JWT may exist only in the secret store and the in-memory HTTP
   request being constructed.
2. Never persist request headers containing `Authorization`.
3. Never return upstream request headers through the VFS.
4. Sanitize upstream error bodies before persistence. Store status code,
   correlation ID when present, and a bounded redacted message only.
5. `Debug` implementations for credential-bearing structures must print a
   fixed redacted marker.
6. Tests must scan all representative public route output for the exact test
   JWT and common bearer/header spellings.

## 8. VFS contract

```text
/apps/near-intents/
├── meta/
│   └── route-contract.json
├── tokens.json
├── settings/
│   ├── api-key
│   └── status.json
└── swaps/
    └── <wallet>/
        ├── new
        ├── latest
        └── <id>/
            ├── request.json
            ├── quote.json
            ├── review_intent.json
            ├── plan.md
            ├── policy_check.json
            ├── approval.json
            ├── outbox.json
            ├── status.json
            ├── receipt.json
            ├── confirm
            ├── refresh
            └── abandon
```

### 8.1 Route behavior

| Route | Operation | Behavior |
|---|---|---|
| `meta/route-contract.json` | read | Static machine-readable route and capability contract |
| `tokens.json` | read | Fetch current 1Click tokens, filter/annotate executable Bloom origins, cache briefly |
| `settings/api-key` | read/write | Report configured state or persist the JWT without echoing it |
| `settings/status.json` | read | Report credential presence, endpoint binding, and supported origin mappings |
| `swaps/<wallet>/new` | read/write | Show input schema or synchronously create the caller-named quote session |
| `swaps/<wallet>/latest` | read | Convenience pointer only; agents must use their caller-supplied session ID |
| `request.json` | read | Canonical user request plus derived origin/refund fields |
| `quote.json` | read | Redacted complete quote, signature, verification result, and hash |
| `review_intent.json` | read | Stable machine-readable action reviewed by the user |
| `plan.md` | read | Human-readable Petal plan, then authoritative Bloom outbox plan after staging |
| `policy_check.json` | read | Preflight results and Bloom policy/outbox warnings when available |
| `approval.json` | read | Current Sealed Approval challenge metadata, never secrets |
| `outbox.json` | read | Outbox ID, chain, state, tx hash, and sanitized receipt summary |
| `status.json` | read | Persisted local/upstream state; no network side effect |
| `receipt.json` | read | Terminal swap outcome and transaction links/hashes |
| `confirm` | write | Prepare, stage, or advance the exact persisted deposit transaction |
| `refresh` | write | Inspect the outbox, submit a mined tx hash once, and poll 1Click status |
| `abandon` | write | Mark a pre-deposit session abandoned; cannot cancel an on-chain deposit |

`new`, `confirm`, `refresh`, and `abandon` are synchronous write routes. A
successful write means its one durable state transition is visible through
`status.json` before the write returns. This read-after-write guarantee avoids
racing `latest` or guessing whether an asynchronous job finished.

## 9. User input

`swaps/<wallet>/new` accepts:

```json
{
  "session_id": "agent-20260714-0001",
  "swap_type": "EXACT_INPUT",
  "origin_asset": "nep141:arb-0x....omft.near",
  "destination_asset": "nep141:sol-....omft.near",
  "amount": "1000000",
  "recipient": "13Q...",
  "slippage_bps": 100,
  "deadline_seconds": 900,
  "quote_waiting_time_ms": 3000,
  "refund_to": null
}
```

Rules:

- `session_id` is caller-generated, 8..=64 ASCII letters, digits, hyphens, or
  underscores. It is reserved with `put-new` before any upstream quote request;
  reuse is rejected before another side effect. Agents address the session by
  this ID and never depend on `latest` for correctness.
- `swap_type` is `EXACT_INPUT` or `EXACT_OUTPUT`.
- `amount` is a canonical positive integer string with no sign, decimal point,
  exponent, or leading zeros except the value `0`; zero is rejected.
- `slippage_bps` is `0..=1000`.
- `deadline_seconds` defaults to `900` and is bounded to `300..=3600`.
- `quote_waiting_time_ms` defaults to `3000` and is bounded to `0..=10000`.
- `origin_asset`, `destination_asset`, and `recipient` are bounded UTF-8
  strings and may not contain control characters.
- `refund_to`, when omitted, is the selected Bloom wallet's EVM address.
- A supplied `refund_to` must be the selected Bloom wallet address in version
  1. Arbitrary refund addresses are rejected to prevent accidental or hostile
  diversion.

The route obtains the wallet address through mediated VFS read. A watch-only
wallet may create dry/local review state but cannot advance to an executable
outbox deposit. In version 1, `new` requests an executable `dry: false` quote;
if the wallet cannot transact, creation fails before making that request.

## 10. Origin-chain and asset resolution

Version 1 recognizes this fixed mapping:

| 1Click `blockchain` | Bloom chain | Expected chain ID |
|---|---|---:|
| `eth` | `ethereum` | 1 |
| `base` | `base` | 8453 |
| `arb` | `arbitrum` | 42161 |
| `op` | `optimism` | 10 |
| `pol` | `polygon` | 137 |
| `bsc` | `bsc` | 56 |
| `avax` | `avalanche` | 43114 |
| `gnosis` | `gnosis` | 100 |

Other EVM chains remain visible as `quote_only` or `unsupported_origin` in
`tokens.json`, but cannot create an executable session until a reviewed Petal
version adds their mapping.

For each executable origin:

1. Find exactly one `/v0/tokens` record whose `assetId` equals
   `origin_asset`.
2. Require the record's `blockchain` to map to a Bloom chain.
3. Call mediated `eth_chainId` and require the expected value.
4. If `contractAddress` is absent/null, treat the asset as native currency.
5. Otherwise require a canonical 20-byte EVM address, non-empty contract code,
   and an `eth_call` to `decimals()` that equals the token record.
6. Require the wallet to have sufficient native gas balance.
7. Require sufficient native or `balanceOf(wallet)` token balance for the
   quote's `amountIn`.

The exact token record used for preparation is persisted with the session.
The signed quote binds `originAsset`, but currently does not bind the token
endpoint's `contractAddress`; therefore the contract address, chain ID, code
presence, and decimals must be prominent in `plan.md` and
`review_intent.json`.

## 11. Quote creation and verification

### 11.1 Creation sequence

1. Validate the local request without network side effects.
2. Resolve and persist the wallet, chain, and origin token record.
3. Reserve the caller-supplied session ID with `put-new` before network work.
4. Acquire the per-session lock and persist state `quoting`.
5. Load the partner JWT from the secret namespace.
6. POST the constrained quote request.
7. Bound and parse the response; retain the original response JSON bytes.
8. Verify the quote signature.
9. Verify the echoed `quoteRequest` matches every execution-relevant field
   sent by the Petal.
10. Validate the quote fields and persist an immutable prepared quote.
11. Render review material and transition to `quoted`.

Any failure persists a sanitized error and transitions to `quote_failed`.
No deposit transaction is staged during quote creation.

### 11.2 Signature algorithm

Match the official TypeScript SDK semantics exactly:

1. Construct the SDK-defined signed request projection. Do not serialize the
   full response object blindly.
2. Construct the dry or non-dry signed quote projection. Version 1 always uses
   the non-dry projection.
3. Reproduce JavaScript object spread in
   `{...signedRequest, ...signedQuote, timestamp}` in that order. Quote-side
   keys overwrite request-side keys even when the quote-side value is
   `undefined`. In particular, overlapping `deadline`,
   `virtualChainRecipient`, `virtualChainRefundRecipient`, and
   `customRecipientMsg` keys are replaced by the quote projection; an
   `undefined` replacement removes the key from the serialized object rather
   than revealing the earlier request value.
4. Reproduce the SDK's falsy coercion before serialization. The SDK converts
   optional numeric `0` and optional empty strings to `undefined` using `?:`
   or `|| undefined`. This applies to `quoteWaitingTimeMs` and every optional
   quote field selected with `|| undefined`, including `timeEstimate`. Required
   amount fields and other fields copied directly are not truthiness-filtered.
5. Omit every resulting `undefined` key, then serialize with deterministic JSON
   key ordering equivalent to
   `json-stable-stringify`.
6. SHA-256 the UTF-8 serialization.
7. Base58-encode the 32-byte digest. The resulting Base58 text, not the raw
   digest, is the Ed25519 message.
8. Decode the response's optional `ed25519:`-prefixed Base58 signature.
9. Verify against the production manager public key:

   `ed25519:reYaWhvwu8Jzo3WUM3zhn6VrhuMEF4eADL17qtRVifc`

The implementation must port the field projection from the official SDK and
pin upstream commit
`ae28ef0348f616dd30c174cb22dd1b1126d8f76b` in a source comment. Tests must
include a valid upstream fixture plus mutations of deposit address, amount,
recipient, refund address, deadline, timestamp, and signature. A production
manager-key rotation requires a reviewed Petal release; the old version fails
closed rather than accepting a runtime-supplied public key.

If the OpenAPI schema gains fields that affect deposits but the signed
projection has not been reviewed, the Petal fails closed rather than assuming
they are harmless.

### 11.3 Executable quote checks

Require all of the following:

- `dry == false`;
- `depositType == ORIGIN_CHAIN`;
- `refundType == ORIGIN_CHAIN`;
- `recipientType == DESTINATION_CHAIN`;
- exact request/response match for assets, requested amount, wallet refund
  address, recipient, swap type, slippage, and request deadline;
- a valid 20-byte EVM `depositAddress`;
- no `depositMemo`;
- non-zero `amountIn` and `amountOut`;
- `minAmountIn <= amountIn` and `minAmountOut <= amountOut`;
- a parseable quote `deadline` later than now plus a configurable safety
  margin, initially 120 seconds; and
- a `timeWhenInactive`, when present, that has not passed.

The transaction deposit amount is always the verified response's `amountIn`.
For `EXACT_OUTPUT`, this deliberately includes the input-side buffer described
by 1Click; unused input is expected to be refunded by the protocol.

## 12. Prepared transaction

The Petal prepares exactly one EVM transaction.

### 12.1 Native origin asset

```text
to        = verified quote.depositAddress
value_wei = verified quote.amountIn
data      = 0x
```

### 12.2 ERC-20 origin asset

```text
to        = validated token.contractAddress
value_wei = 0
data      = ABI encode transfer(verified quote.depositAddress,
                                verified quote.amountIn)
```

The function selector must be `a9059cbb`. Address and amount are ABI-encoded
locally with independently tested vectors.

The prepared transaction record includes a canonical digest over:

- package/app identity;
- session ID and wallet;
- Bloom chain and expected chain ID;
- origin asset and token contract/native marker;
- deposit address;
- amount;
- transaction `to`, `value`, and calldata;
- quote correlation ID, quote hash, signature, and deadline; and
- destination asset, recipient, minimum output, and refund address.

Once prepared, these fields are immutable. A changed quote creates a new
session ID.

## 13. Review material

Before staging, `plan.md` must clearly show:

- that this is a 1Click swap and that funds are temporarily transferred to
  the 1Click swapping flow;
- Bloom wallet and origin chain/chain ID;
- input token symbol, asset ID, contract/native marker, decimals, exact deposit
  amount, and USD estimate as informational only;
- destination asset, recipient, quoted output, minimum output, and estimated
  execution time;
- slippage, refund address, refund fee, withdrawal fee, and deadlines;
- signed deposit address and quote verification status;
- quote correlation ID and local quote digest;
- the exact EVM `to`, value, and decoded operation;
- whether mainnet broadcast is currently expected to require Bloom opt-in; and
- warnings that NEAR Intents has no testnet and that status may take minutes.

`review_intent.json` contains the same facts as typed fields and the prepared
artifact digest. It must not contain the JWT or authorization headers.

After `tx_stage`, the Bloom-generated outbox `plan.md` becomes authoritative.
The route may prepend a short NEAR Intents summary, but it must preserve the
outbox plan verbatim and label it as Bloom's transaction plan.

## 14. Confirmation protocol

`confirm` accepts only:

- `confirm`
- `y`
- `{"confirm":true}`
- `{"confirm":true,"acknowledge_warnings":true}`

One write advances at most one durable transition:

1. `quoted` -> perform final quote/preflight checks, persist prepared
   transaction, return `prepared`.
2. `prepared` -> set `staging_started`, call `tx_stage`, persist the outbox ID
   and Bloom plan, return `staged`.
3. `staged` -> call `tx_confirm`. Persist either `approval_required` or
   `deposit_broadcast_pending`.
4. `approval_required` -> retry the same outbox confirmation after approval;
   never reconstruct or restage transaction bytes.
5. `deposit_broadcast_pending` -> inspect only; confirmation does not submit a
   second transaction.

A separate write between preparation, staging, and confirmation is required so
the user can inspect the authoritative plan.

The route must set a durable `staging_started` ambiguity marker before calling
`tx_stage`. If staging may have succeeded but its outbox ID was not persisted,
the Petal refuses automatic restaging and reports manual recovery instructions.

## 15. Refresh and settlement protocol

`refresh` accepts `refresh` or `{"refresh":true}` and performs bounded work:

1. Inspect the owned Bloom outbox entry when one exists.
2. Persist its state, tx hash, and sanitized receipt.
3. If the outbox has no transaction hash yet, return without polling 1Click;
   the executable local state remains unchanged.
4. If the outbox has a mined/sent tx hash and no successful submit receipt,
   atomically claim a submit lock, POST `/v0/deposit/submit`, and persist the
   response. A timeout after sending is marked `submit_ambiguous`; retrying is
   permitted because submission is advisory and keyed by deposit address plus
   tx hash, but the ambiguity is retained in history.
5. GET `/v0/status` with the exact persisted deposit address.
6. Verify the embedded `quoteResponse` signature and require it to match the
   persisted quote hash, deposit address, asset pair, recipient, and refund
   address. Live status responses may omit `quoteResponse.correlationId`, and
   the status envelope `correlationId` is an independent response identifier
   that can differ from the original quote ID. Because correlation ID is not in
   the SDK signed projection, reconstruct a missing nested value from the
   persisted verified quote. If the nested value is present, require it to
   equal the persisted quote ID. Do not use the envelope response identifier as
   a quote binding.
   The live status endpoint may also omit the originally signed nonzero
   `quoteWaitingTimeMs`; restore only that absent field from the persisted,
   already verified quote before recomputing the SDK projection. Never
   overwrite a present status value. Normalized `appFees: []` and omitted
   `insured` remain outside the SDK signed projection and execution behavior.
7. Persist the upstream state and sanitized `swapDetails`.
8. Render `receipt.json` when upstream status is terminal.

HTTP, decoding, signature, or correlation failures update `last_error` but do
not transition away from the last valid executable state.

`status.json` is a pure read of the last persisted state. It must not poll on
read.

Terminal interpretation:

| Upstream status | Local state |
|---|---|
| `SUCCESS` | `settled_success` |
| `REFUNDED` | `settled_refunded` |
| `FAILED` | `settled_failed` |
| `INCOMPLETE_DEPOSIT` | `deposit_incomplete` (non-terminal until upstream changes or deadline policy says otherwise) |

`receipt.json` includes the origin transaction hash, intent hashes, NEAR
transaction hashes, destination transaction hashes and explorer URLs, actual
amounts, fees, refund amount/reason, and timestamps when supplied. It never
claims destination success based only on the origin EVM receipt.

## 16. State model

Store one JSON session record in the `state` namespace at:

`swaps/<wallet>/<id>/session.json`

Large raw upstream responses may be stored separately under the same prefix.
The public routes render allowlisted projections rather than returning private
store blobs directly.

Minimum session fields:

```text
schema_version
id
wallet
wallet_address
created_ms
updated_ms
state
request
origin_token
bloom_chain
expected_chain_id
raw_quote_key
quote_correlation_id
quote_hash
quote_signature
quote_verified
quote_deadline_ms
prepared_transaction
prepared_digest
staging_started
outbox_id
outbox_state
origin_tx_hash
approval
deposit_submit_state
upstream_status
upstream_updated_at
swap_details
outbox_receipt
last_error
history[]
```

State transitions are append-recorded in a bounded history with timestamp,
prior state, next state, and a non-secret reason. Store writes are atomic.
Locks use `put-new` and compare-and-delete with random ownership tokens.
Every confirm, refresh, and abandon invocation holds the per-session lock for
its entire load/side-effect/save transition. This prevents duplicate staging
and last-writer-wins loss of an outbox ID or transaction hash.

The route must tolerate daemon restart at every persisted state. It must never
depend on in-memory state for correctness.

## 17. State machine

```text
creating
  -> quoting
  -> quoted
  -> prepared
  -> staging_started
  -> staged
  -> approval_required
  -> deposit_broadcast_pending
  -> deposit_sent
  -> pending_deposit | known_deposit | processing | deposit_incomplete
  -> settled_success | settled_refunded | settled_failed
```

Failure/administrative states:

```text
quote_failed
quote_expired
preflight_failed
staging_ambiguous
deposit_failed
submit_ambiguous
abandoned
```

No transition from `abandoned` is allowed. `abandon` is rejected after an
outbox transaction has been confirmed or broadcast because it cannot cancel
the external transfer.

## 18. Idempotency and recovery invariants

1. A caller-supplied session ID identifies one immutable quote and one
   immutable prepared EVM transaction; it is reserved before network work.
2. A session owns at most one outbox ID.
3. Once an outbox ID exists, the component never calls `tx_stage` again.
4. Once an origin tx hash exists, the component never confirms another outbox
   entry for the session.
5. Deposit submit uses the exact persisted deposit address and origin tx hash.
6. Status polling uses the exact persisted deposit address and memo.
7. A terminal upstream state cannot regress. Later contradictory responses are
   retained as an error and do not overwrite the terminal receipt.
8. Quote expiry is checked at creation, preparation, staging, and confirmation.
9. A route error after a potentially successful external side effect must be
   represented as ambiguous, never automatically repeated as though it failed.
10. Locks have bounded expiry and ownership tokens; only their owner may
    release them.

## 19. Security invariants

### 19.1 Quote and destination integrity

- No unsigned or invalidly signed quote can reach `tx_stage`.
- The deposit address, amount, assets, recipient, refund address, deadlines,
  and fee fields displayed for review come from the persisted verified quote.
- The prepared transaction is derived once from that quote and persisted.
- Confirm input is only an acknowledgement; it cannot provide transaction
  fields.

### 19.2 Bloom trust boundary

The Petal is responsible for interpreting 1Click data and constructing the
correct transaction. Bloom structurally enforces package provenance, outbox
ownership, policy, approval, signing, and broadcast controls, but does not
independently understand NEAR Intents. The Petal's review material must be
treated as a security-critical representation.

### 19.3 Network and parser safety

- HTTPS only, exact host, exact method/path allowlist.
- Maximum response size: 8 MiB host ceiling and a stricter 2 MiB application
  ceiling for tokens/quote/status responses.
- Maximum 10,000 token records retained; reject excess.
- Bounded strings, arrays, history, error messages, and receipt fields.
- Reject duplicate JSON keys in security-critical quote fields if the chosen
  parser otherwise accepts last-value wins.
- Never follow a redirect to an undeclared host; Bloom revalidates redirects.
- Treat HTTP 429 and 5xx as retryable status errors with bounded diagnostic
  text. Do not retry automatically inside one route invocation.

### 19.4 Financial safety

- Use `amountIn` exactly; never floating-point arithmetic for token amounts.
- Display USD values as untrusted informational strings only.
- Verify chain ID immediately before staging and confirmation.
- Check balance immediately before staging; Bloom's outbox simulation and
  policy remain authoritative.
- Mainnet broadcasting remains subject to Bloom's global and per-chain opt-in.
- No live-network test is part of normal CI because NEAR Intents has no
  testnet. Manual validation uses explicitly funded low-value wallets.

## 20. Modules

Suggested focused modules under `route/src/`:

```text
api.rs                 typed 1Click HTTP client and sanitized errors
api_types.rs           request/response DTOs for the supported subset
quote_signature.rs     stable JSON projection, SHA-256, Base58, Ed25519
assets.rs              token cache, origin mapping, chain/token checks
evm.rs                 ERC-20 ABI encoding and preflight calls
settings.rs            JWT provisioning and redacted status
session.rs             persisted schema, locks, history, state transitions
workflow.rs            create/prepare/stage/confirm/refresh orchestration
render.rs              shared plan/review/receipt renderers used by routes
redaction.rs           bounded upstream errors and public-output safeguards
runtime_config.rs      endpoint binding access and fixed origin metadata
```

Expected protocol dependencies are `serde`/`serde_json`, `sha2`, `bs58`,
`ed25519-dalek`, and Alloy ABI/primitives crates already proven by the
Polymarket Petal. The signed quote projection should serialize a `BTreeMap` (or
an equivalently ordered dedicated structure) containing only the SDK-selected
fields. Build and merge the request and quote projections before omitting
undefined values: a quote-side absent/falsy optional value must remove an
overlapping request-side key. A plain `Option::None` plus `skip_serializing_if`
on two independently serialized Rust structs is insufficient. Explicitly apply
the SDK's falsy-to-undefined rules, then omit undefined keys. Amounts remain
strings and basis-point/time values remain integers. Do not add a general
JavaScript runtime merely to reproduce `json-stable-stringify`.

Route files remain endpoint controllers: extract params, choose the operation,
and call focused workflow/domain functions. Avoid a stringly typed central
route dispatcher.

## 21. Testing requirements

### 21.1 Unit tests

- Canonical positive integer parser and all bounds.
- Every supported/unsupported chain mapping and chain-ID mismatch.
- Native and ERC-20 asset classification.
- ERC-20 `transfer`, `balanceOf`, and `decimals` calldata vectors.
- Exact quote request construction with prohibited optional fields absent.
- Stable-JSON ordering compatibility.
- SDK-compatible omission for `quoteWaitingTimeMs: 0`, `timeEstimate: 0`, and
  empty optional strings.
- Quote-side overwrite/removal of overlapping request projection keys,
  including the absent quote-deadline case.
- Official valid quote-signature fixture.
- Signature rejection after mutating every execution-critical field.
- Malformed Base58, wrong key, wrong signature length, missing fields,
  duplicate keys, and oversized response.
- Unknown fields are fatal in the top-level quote envelope, `quoteRequest`, and
  `quote`, but tolerated in token records and sanitized `swapDetails`.
- Quote deadline and safety-margin boundaries.
- Plan and review renderers include every required financial field.
- JWT validation, replacement, deletion if implemented, and redacted reads.
- Upstream error redaction.
- All state-machine transitions and forbidden regressions.

### 21.2 Workflow tests with mocked host imports

- Missing JWT fails before quote HTTP.
- Invalid wallet/chain/token fails before quote HTTP.
- Valid quote persists raw evidence and never stages during creation.
- Tampered quote never stages.
- Prepare, stage, inspect, approval-required, retry, sent, and mined flow.
- Mutation after preparation is rejected.
- Persist-before-effect ambiguity markers survive simulated traps.
- Restart from every durable state resumes without duplicate staging.
- Submit hash is called with exact persisted values and is idempotent.
- Status response embedded quote is reverified.
- Success, refund, failure, incomplete deposit, 404, 429, and 5xx handling.
- Terminal-state regression is rejected.

### 21.3 Route/package tests

- Every declared route exports the complete route-file world.
- Imported host interfaces exactly match Bloom-owned WIT.
- No route component imports `bloom:sign/signing@0.1.0` or `@0.2.0`.
- Manifest imports are covered by the capability ceiling.
- Undeclared hosts, methods, paths, and redirects are denied.
- Public VFS sweep proves no JWT, bearer header, raw secret-store path, or
  unredacted upstream credential appears.
- `settings/api-key` remains non-readable after write.
- Built artifacts are WASM components, not core WASM modules.

### 21.4 Live acceptance

Live acceptance is manual and explicitly gated. It uses the smallest practical
amount on a supported EVM chain and proves:

1. API key persists across daemon restart.
2. Quote verifies.
3. Plan names the exact deposit address and amount.
4. Bloom policy/approval gates confirmation.
5. Origin transaction hash is submitted.
6. Status reaches `SUCCESS` or a correctly represented refund/failure state.
7. Receipt contains both origin and destination evidence.

No test may assume a testnet 1Click environment exists.

## 22. Build and validation

Required developer commands:

```sh
cargo test --manifest-path route/Cargo.toml
scripts/build.sh
scripts/validate.sh
```

`scripts/validate.sh` must at minimum:

- run route and framework tests;
- build all route components;
- invoke Bloom's package validator/build command when a local Bloom binary is
  available;
- scan tracked files for generated WASM and target directories;
- scan representative public fixtures for credential leakage; and
- verify `petal.toml` contains no broader network or signing capability than
  this specification permits.

## 23. Implementation sequence

1. Scaffold the package, local framework, WIT, xtask, build scripts, manifest,
   static routes, and package-validation smoke test.
2. Implement settings, persistent JWT storage, redacted status, and network
   policy tests.
3. Implement API DTOs, token discovery, fixed chain mapping, and EVM preflight.
4. Port and test official quote-signature verification.
5. Implement session persistence, locks, state machine, quote creation, and
   review rendering.
6. Implement immutable transaction preparation and generic outbox staging,
   confirmation, inspection, and approval projection.
7. Implement deposit submission, status refresh, terminal receipts, and
   recovery behavior.
8. Complete secret/public-output audit, package build, mocked end-to-end smoke,
   and documentation.
9. Run a separately approved, low-value mainnet acceptance swap.

## 24. Acceptance criteria

The Petal is implementation-complete when:

- it installs as a valid v2 package and mounts at `/apps/near-intents`;
- a JWT written once remains usable after daemon restart without being readable
  through the VFS;
- an EVM-origin exact-input or exact-output request produces a valid signed
  quote and review tree;
- a quote with any execution-critical mutation is rejected;
- confirmation stages exactly one native or ERC-20 transaction through Bloom's
  generic outbox;
- the outbox plan and Sealed Approval lifecycle are visible and retryable;
- a mined origin hash is submitted once and status can be refreshed to a
  terminal receipt;
- unsupported/non-EVM origins fail before funds can move;
- all public output and persisted diagnostics pass the credential-redaction
  sweep; and
- all unit, workflow, route, build, and validation tests pass.

## 25. References

- NEAR Intents 1Click OpenAPI:
  `https://1click.chaindefuser.com/docs/v0/openapi.yaml`
- 1Click quickstart:
  `https://docs.near-intents.org/integration/distribution-channels/1click-api/quickstart`
- API authentication:
  `https://docs.near-intents.org/integration/distribution-channels/1click-api/authentication`
- Quote verification:
  `https://docs.near-intents.org/integration/distribution-channels/1click-api/verify-quote-signature`
- Official TypeScript verifier:
  `https://github.com/defuse-protocol/one-click-sdk-typescript/blob/main/src/quote-signature.ts`
- Supported assets:
  `https://docs.near-intents.org/resources/asset-support`
- Supported chains:
  `https://docs.near-intents.org/resources/chain-support`
- Bloom local Petal apps:
  `../../bloom/docs/guides/local-petal-apps-v2.md`
- Bloom Machine and Petal trust boundary:
  `../../bloom/docs/architecture/Bloom Machine + Petals.md`
- Polymarket v2 Petal reference:
  `../../bloom-petal-polymarket/`
