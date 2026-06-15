# Public, open-source repository

`smbcloud-cli` is published on GitHub as **open source**. Everything you commit here — code, comments, docs, and `.agents/skills/*` — is **world-readable and permanent** (git history outlives any later deletion). Before writing anything, ask: "is this safe on a public repo forever?"

Do **not** add internal smbCloud infrastructure detail or secrets:

- server hostnames/IPs and operational endpoints beyond what the CLI already targets in source — e.g. account-scoped SSH key names (`id_<n>@smbcloud`), `api-1.smbcloud.xyz`, internal health ports
- account/user IDs and which user or tenant owns which project
- real customer/app domains, PM2 process names, the production port → app → domain registry
- workspace/project IDs and `frontend_app_id` / `deploy_repo_id` values
- secrets/config: API keys, tokens, `.env` values, connection strings, real auth/CORS origins
- commands that read local credentials to enumerate the API (e.g. `cat ~/.smb/token | curl …`)
- incident logs or examples that name real apps, tenants, customers, or dated rollouts

Keep docs and skills **generic** — describe how the tool behaves, using placeholders (`example.com`, `<app>`, `<source>`, `<port>`, `<n>`). The base API host (`api.smbcloud.xyz`) is already in the source, so it is not a new leak; the items above are. Fleet-specific operational detail and the internal deploy reference live only in the private `smbcloud` repo.

---

# Rust coding guidelines

- Prioritize code correctness and clarity. Speed and efficiency are secondary priorities unless otherwise specified.
- Do not write organizational or comments that summarize the code. Comments should only be written in order to explain "why" the code is written in some way in the case there is a reason that is tricky / non-obvious.
- Prefer implementing functionality in existing files unless it is a new logical component. Avoid creating many small files.
- Avoid using functions that panic like `unwrap()`, instead use mechanisms like `?` to propagate errors.
- Be careful with operations like indexing which may panic if the indexes are out of bounds.
- Never silently discard errors with `let _ =` on fallible operations. Always handle errors appropriately:
  - Propagate errors with `?` when the calling function should handle them
  - Use `.log_err()` or similar when you need to ignore errors but want visibility
  - Use explicit error handling with `match` or `if let Err(...)` when you need custom logic
  - Example: avoid `let _ = client.request(...).await?;` - use `client.request(...).await?;` instead
- When implementing async operations that may fail, ensure errors propagate to the UI layer so users get meaningful feedback.
- Never create files with `mod.rs` paths - prefer `src/some_module.rs` instead of `src/some_module/mod.rs`.
- When creating new crates, prefer specifying the library root path in `Cargo.toml` using `[lib] path = "...rs"` instead of the default `lib.rs`, to maintain consistent and descriptive naming (e.g., `gpui.rs` or `main.rs`).
- Avoid creative additions unless explicitly requested
- Use full words for variable names (no abbreviations like "q" for "queue")
- Use variable shadowing to scope clones in async contexts for clarity, minimizing the lifetime of borrowed references.
  Example:
  ```rust
  executor.spawn({
      let task_ran = task_ran.clone();
      async move {
          *task_ran.borrow_mut() = true;
      }
  });
  ```
