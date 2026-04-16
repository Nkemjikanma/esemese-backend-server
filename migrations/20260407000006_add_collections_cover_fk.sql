ALTER TABLE collections
    ADD CONSTRAINT fk_collections_cover_photo
    FOREIGN KEY (cover_photo_id) REFERENCES photos(id) ON DELETE SET NULL;
