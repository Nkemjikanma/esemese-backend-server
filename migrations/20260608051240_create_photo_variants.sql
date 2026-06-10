CREATE TYPE variant_format as ENUM ('avif', 'webp','jpeg');
-- Add migration script here
CREATE TABLE variants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    photo_id UUID NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    s3_key VARCHAR(512) NOT NULL UNIQUE,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    format variant_format NOT NULL,
    byte_size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_variants_photo_size_format UNIQUE (photo_id, width, format)
);

CREATE INDEX idx_variants_photo_id ON variants(photo_id);