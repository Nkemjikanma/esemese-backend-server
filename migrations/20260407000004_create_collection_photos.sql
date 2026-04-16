CREATE TABLE collection_photos (
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    photo_id UUID NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (collection_id, photo_id)
);

CREATE INDEX idx_collection_photos_collection_id ON collection_photos(collection_id);
CREATE INDEX idx_collection_photos_photo_id ON collection_photos(photo_id);
