import { sql } from 'drizzle-orm';
import { int, sqliteTable, text } from 'drizzle-orm/sqlite-core';

export const logs = sqliteTable('logs', {
  id: int().primaryKey({ autoIncrement: true }),
  message: text({ mode: 'json' }).notNull(),
  createdAt: int({ mode: 'timestamp' })
    .notNull()
    .default(sql`(unixepoch())`),
});
