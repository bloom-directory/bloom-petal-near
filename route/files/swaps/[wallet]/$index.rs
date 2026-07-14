petal::route_file!(spec: petal::store_dir_spec().caps(&["bloom:store"]), ctx_list: |ctx: &petal::Ctx| {let wallet=petal::param(ctx,"wallet")?;crate::workflow::session_children(wallet)});
