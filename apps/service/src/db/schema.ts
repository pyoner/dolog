import { int, sqliteTable, text } from 'drizzle-orm/sqlite-core';

export const keys = sqliteTable('keys', {
  id: int().primaryKey({ autoIncrement: true }),
  value: text()
    .unique()
    .$default(() => crypto.randomUUID()),
  name: text(),
  created_at: int({ mode: 'timestamp_ms' }).$default(() => new Date()),
});

export const logs = sqliteTable('logs', {
  id: int().primaryKey({ autoIncrement: true }),
  message: text({ mode: 'json' }),
  created_at: int({ mode: 'timestamp_ms' }).$default(() => new Date()),
});
