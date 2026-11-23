CREATE TABLE channel_settings (
    channel_id CHAR(36) CHARACTER SET ascii COLLATE ascii_bin PRIMARY KEY,
    senryu_enabled BOOLEAN NOT NULL,
    updated_at DATETIME(6) NOT NULL
);

CREATE TABLE senryu (
    message_id CHAR(36) CHARACTER SET ascii COLLATE ascii_bin PRIMARY KEY,
    channel_id CHAR(36) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    verse TEXT NOT NULL,
    detected_at DATETIME(6) NOT NULL,
    INDEX senryu_newest (detected_at, message_id)
) DEFAULT CHARACTER SET utf8mb4;
