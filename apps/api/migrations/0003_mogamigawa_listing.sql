ALTER TABLE mogamigawa_responses
    ADD COLUMN channel_id CHAR(36) CHARACTER SET ascii COLLATE ascii_bin NULL,
    ADD COLUMN verse TEXT CHARACTER SET utf8mb4 NULL,
    ADD COLUMN detected_at DATETIME(6) NULL,
    ADD COLUMN listed BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN response_started BOOLEAN NOT NULL DEFAULT TRUE,
    ADD INDEX mogamigawa_newest (listed, detected_at, message_id);
