petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:store", "bloom:tx.outbox", "bloom:chain", "bloom:vfs.read"]),
    read: |_ctx: &petal::Ctx| {
        petal::DispatchResponse::Read(
            b"Write confirm, y or {\"confirm\":true} to advance one step; status.json shows the new state. To accept warnings shown in the staged plan, write {\"confirm\":true,\"acknowledge_warnings\":true}.
1. quoted -> prepared: builds the deposit transaction.
2. prepared -> staged: stages it in the wallet outbox for the owner to review. Stop here unless the user asked to submit.
3. staged -> approval_required or deposit_broadcast_pending: asks the owner to approve, or broadcasts if no approval is needed.
4. approval_required -> deposit_broadcast_pending: after the owner approves, write confirm again. Repeat while the state is approval_required: a wallet policy approval can come before the transaction approval.
Steps 1-4 are refused once the quote deadline (quote.deadline in quote.json) is less than two minutes away. In deposit_broadcast_pending or deposit_sent, confirm only inspects the existing outbox entry, even after the deadline.
"
            .to_vec(),
        )
    },
    write: |ctx: &petal::Ctx, body: &[u8]| {
        let wallet = match petal::wallet_param(ctx) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let id = match petal::param(ctx, "id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        let mut host = crate::workflow::BloomHost;
        match crate::workflow::confirm(&mut host, wallet, id, body) {
            Ok(()) => petal::DispatchResponse::Write,
            Err(error) => petal::error(-4, crate::redaction::sanitize_message(&error)),
        }
    }
);
