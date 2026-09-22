petal::route_file!(
    spec: petal::static_dir_spec(),
    ctx_list: |ctx: &petal::Ctx| {
        petal::wallet_param(ctx)?;
        Ok(vec![petal::writable("venue.toml")])
    }
);
