petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| crate::session_view(ctx,"review"));
