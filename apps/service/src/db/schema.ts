import { int, sqliteTable, text } from 'drizzle-orm/sqlite-core';

export const keys = sqliteTable('keys', {
  id: text().primaryKey(),
  name: text().notNull(),
});

export const logs = sqliteTable('logs', {
  id: int().primaryKey({ autoIncrement: true }),
});
