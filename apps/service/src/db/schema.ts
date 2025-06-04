import { int, sqliteTable, text } from 'drizzle-orm/sqlite-core';

export const keys = sqliteTable('keys', {
  id: int().primaryKey({ autoIncrement: true }),
  value: text()
    .unique()
    .$default(() => crypto.randomUUID()),
  name: text(),
  createdAt: int({ mode: 'timestamp_ms' }).$default(() => new Date()),
});

export const logs = sqliteTable('logs', {
  id: int().primaryKey({ autoIncrement: true }),
  message: text({ mode: 'json' }),
  createdAt: int({ mode: 'timestamp_ms' }).$default(() => new Date()),
});
