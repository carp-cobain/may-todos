ALTER TABLE stories
  ADD CONSTRAINT check_name_length
  CHECK (char_length(name) >= 3 AND char_length(name) <= 100);
