create table if not exists users (
    user_id TEXT primary key
);

create table if not exists auth_tokens (
    -- TODO: I'm not sure if this should allow users to have multiple logins.
    user_id TEXT references users(user_id) not null,
    token_hash BLOB not null,
    expires_at TEXT
);

create table if not exists discord_oauth_users (
    discord_id TEXT primary key,
    linked_to_user_id TEXT references users(user_id) not null,
    -- this should probably be nullable?
    refresh_token TEXT,
    -- given these expire, they should probably be pruned?
    access_token TEXT,
    -- iso86001 timestamp
    expires_at TEXT
);
