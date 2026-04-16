CREATE TYPE upload_status AS ENUM ('initiated', 'uploaded', 'confirmed', 'failed');

CREATE TABLE uploads (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    original_filename VARCHAR(255) NOT NULL,
    s3_key VARCHAR(512) NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    status upload_status NOT NULL DEFAULT 'initiated',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_uploads_status ON uploads(status);
CREATE INDEX idx_uploads_created_at ON uploads(created_at);
