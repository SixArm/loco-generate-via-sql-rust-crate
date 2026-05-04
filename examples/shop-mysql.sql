-- A small e-commerce schema in MySQL.
-- Demonstrates: INT UNSIGNED (`unsigned`), BIGINT UNSIGNED (`big_unsigned`),
-- TINYINT(1) → bool, DECIMAL → decimal, foreign key rule 1, MEDIUMTEXT → text.

CREATE TABLE customers (
    id INT UNSIGNED NOT NULL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    active TINYINT(1) NOT NULL DEFAULT 1,
    notes MEDIUMTEXT
);

CREATE TABLE products (
    id INT UNSIGNED NOT NULL PRIMARY KEY,
    sku VARCHAR(64) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,
    stock INT UNSIGNED NOT NULL DEFAULT 0,
    active TINYINT(1) NOT NULL DEFAULT 1
);

CREATE TABLE orders (
    id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    customer_id INT UNSIGNED NOT NULL REFERENCES customers(id),
    total DECIMAL(10, 2) NOT NULL,
    placed_at DATETIME NOT NULL,
    paid TINYINT(1) NOT NULL DEFAULT 0
);

CREATE TABLE order_items (
    id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    order_id BIGINT UNSIGNED NOT NULL REFERENCES orders(id),
    product_id INT UNSIGNED NOT NULL REFERENCES products(id),
    quantity INT UNSIGNED NOT NULL,
    unit_price DECIMAL(10, 2) NOT NULL
);
