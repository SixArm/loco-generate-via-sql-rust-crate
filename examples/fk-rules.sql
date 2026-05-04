-- Demonstrates all three foreign-key emission rules.
--
-- Rule 1: column ends in `_id` AND prefix matches singularized target table.
--   author_id REFERENCES authors(id)  →  author:references
--
-- Rule 2: column ends in `_id` BUT prefix does not match the target.
--   owner_id REFERENCES users(id)     →  user:references:owner_id
--
-- Rule 3: column does not end in `_id`.
--   author REFERENCES users(id)       →  user:references:author

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE posts (
    id SERIAL PRIMARY KEY,

    -- Rule 1: prefix `author` matches depluralized `authors`.
    author_id INT NOT NULL REFERENCES authors(id),

    -- Rule 2: prefix `owner` does not match depluralized `users` (= `user`).
    owner_id INT NOT NULL REFERENCES users(id),

    -- Rule 3: column does not end in `_id`.
    editor INT NOT NULL REFERENCES users(id),

    title TEXT NOT NULL
);
