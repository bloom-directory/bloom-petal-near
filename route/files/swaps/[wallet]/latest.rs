petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| match petal::param(ctx,"wallet"){Ok(w)=>crate::workflow::latest_route(w),Err(e)=>e});
