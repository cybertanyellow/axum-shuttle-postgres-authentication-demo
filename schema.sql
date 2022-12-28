CREATE TABLE IF NOT EXISTS users (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    username text NOT NULL UNIQUE, -- CHECK (name <> '')
    password text NOT NULL,
    email text NOT NULL,
    phone text NOT NULL,
    role_id integer REFERENCES role (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS role (
    id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    permission text NOT NULL UNIQUE -- ADM,GM,Maintenance,Commissioner,JSHall
);

CREATE TABLE IF NOT EXISTS sessions (
    session_token BYTEA PRIMARY KEY,
    user_id integer REFERENCES users (id) ON DELETE CASCADE
);
