-- Add migration script here
 CREATE VIEW published_photos AS SELECT id, title, description, category, featured, width, height, blurhash, created_at FROM photos WHERE status = 'ready';