CREATE TYPE photo_status AS ENUM  ('initiated', 'uploaded', 'processing', 'ready', 'failed');

CREATE TABLE photos (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(255),
    description TEXT,
    category VARCHAR(100),
    featured BOOLEAN NOT NULL DEFAULT FALSE,
    original_filename VARCHAR(255) NOT NULL,
    status photo_status NOT NULL DEFAULT 'initiated',
    blurhash TEXT,
    s3_key VARCHAR(512) NOT NULL UNIQUE,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    width INTEGER,
    height INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_photos_category ON photos(category);
CREATE INDEX idx_photos_featured ON photos(featured) WHERE featured = TRUE;
CREATE INDEX idx_photos_created_at ON photos(created_at DESC);
CREATE INDEX idx_photos_published ON photos(created_at DESC) WHERE status = 'ready';