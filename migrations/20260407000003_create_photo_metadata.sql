CREATE TABLE photo_metadata (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    photo_id UUID NOT NULL UNIQUE REFERENCES photos(id) ON DELETE CASCADE,
    camera VARCHAR(255),
    lens VARCHAR(255),
    iso INTEGER,
    aperture VARCHAR(50),
    shutter_speed VARCHAR(50),
    focal_length VARCHAR(50),
    location VARCHAR(500),
    taken_at TIMESTAMPTZ
);

CREATE INDEX idx_photo_metadata_photo_id ON photo_metadata(photo_id);
