-- Migration 006: indexes for the "Recently added" / "Recently updated" sorts.
-- LibraryView's sort menu offers created_at DESC and updated_at DESC, which
-- queries.rs feeds straight into ORDER BY ... LIMIT/OFFSET. Without an index,
-- every page fetch re-sorts the entire images table (tens of thousands of rows)
-- and scrolling issues hundreds of such full sorts. These turn each page into
-- an index walk.
CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at);
CREATE INDEX IF NOT EXISTS idx_images_updated_at ON images(updated_at);
