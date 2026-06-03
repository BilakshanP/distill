-- RLS for tenant isolation.
-- Policies use session variable set per-transaction by the application.
-- Falls back to default tenant (single-tenant mode) if unset.

ALTER TABLE questions ENABLE ROW LEVEL SECURITY;
ALTER TABLE answers ENABLE ROW LEVEL SECURITY;
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE config ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_questions ON questions USING (
    tenant_id = COALESCE(
        NULLIF(current_setting('app.current_tenant', true), '')::uuid,
        '00000000-0000-0000-0000-000000000000'::uuid
    )
);
CREATE POLICY tenant_answers ON answers USING (
    tenant_id = COALESCE(
        NULLIF(current_setting('app.current_tenant', true), '')::uuid,
        '00000000-0000-0000-0000-000000000000'::uuid
    )
);
CREATE POLICY tenant_users ON users USING (
    tenant_id = COALESCE(
        NULLIF(current_setting('app.current_tenant', true), '')::uuid,
        '00000000-0000-0000-0000-000000000000'::uuid
    )
);
CREATE POLICY tenant_config ON config USING (
    tenant_id = COALESCE(
        NULLIF(current_setting('app.current_tenant', true), '')::uuid,
        '00000000-0000-0000-0000-000000000000'::uuid
    )
);

ALTER TABLE questions FORCE ROW LEVEL SECURITY;
ALTER TABLE answers FORCE ROW LEVEL SECURITY;
ALTER TABLE users FORCE ROW LEVEL SECURITY;
ALTER TABLE config FORCE ROW LEVEL SECURITY;
