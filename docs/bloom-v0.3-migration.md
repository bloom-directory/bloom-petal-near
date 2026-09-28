# Explicit numbered-account route contract

The package retains the pinned canonical Petal SDK and CLI at
`2beed2ff344ce2b0c112e07096027e1ae0404007`. The Bloom host must support
adjacent `[wallet]/[index]` route captures and uniform private account storage.
Old hosts and unmodified custom packages are incompatible with this contract.

Operations live at `swaps/<wallet>/<index>/{new,latest,<id>/...}` and credentials
at `settings/<wallet>/<index>/{api-key,status.json}`. Public `tokens.json` and
`meta/` remain unscoped. No `[account] aware` manifest switch or global route-root
rewrite is used. Bloom matches the captures against its live authenticated
account projection and supplies trusted `bloom.wallet` and `bloom.account`.
The request body cannot select or override an account. Missing trusted context
is rejected before quote creation. Bloom synthesizes parent directory children from the authenticated core wallet
and numbered-account projections; guests need no broad wallet-listing authority.

Canonical identity is `wallets/<wallet>/<index>/address.evm`; balance and exact
outbox artifacts live under that account's `chains/<chain>/`. Sessions persist
the selected account number and address and revalidate them before side effects.
Address syntax alone is not proof of signing authority. The host transaction
outbox owns Broker authorization, simulation, policy, signing, and broadcast.
No wallet-root `kind` leaf is used. Missing account identity never falls back
to a wallet-root address or another numbered account.

Every numbered account, including 0, uses a uniform private store. This package
performs no session migration and has no legacy account-0 store fallback.
Historical unbound sessions cannot execute automatically.

Existing installed state must be retained until pending operations are audited.
NEAR's durable session includes the deposit address, signed quote, quote hash,
refund recipient, correlation ID, prepared deposit, and exact outbox identifiers.
Dropping that state can remove the information needed to reconcile an ambiguous
deposit or query settlement, although origin refunds are bound to the persisted
wallet address. Core outbox records remain independently inspectable. A storage
cutover requires deliberate manual recovery of pending sessions; this change
does not remove any installed state and does not claim a funds migration.

Validation uses mocked unit tests and package builds, followed by an isolated
Bloom smoke test when the revised host is available. No live-money swap or
broadcast is part of validation.
