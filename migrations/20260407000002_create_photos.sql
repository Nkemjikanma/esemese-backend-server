CREATE TABLE photos (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(255),
    description TEXT,
    category VARCHAR(100),
    featured BOOLEAN NOT NULL DEFAULT FALSE,
    original_filename VARCHAR(255) NOT NULL,
    s3_key VARCHAR(512) NOT NULL UNIQUE,
    thumbnail_s3_key_small VARCHAR(512),
    thumbnail_s3_key_medium VARCHAR(512),
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
