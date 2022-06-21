-- Remove salt from users. Use PHC string format for password hash instead
ALTER TABLE users DROP COLUMN salt;