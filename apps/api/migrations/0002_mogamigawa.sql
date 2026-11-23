ALTER TABLE channel_settings
    ADD COLUMN mogamigawa_enabled BOOLEAN NOT NULL DEFAULT TRUE;

CREATE TABLE mogamigawa_responses (
    message_id CHAR(36) CHARACTER SET ascii COLLATE ascii_bin PRIMARY KEY
);
