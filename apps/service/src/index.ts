import { DurableObject } from 'cloudflare:workers';
import { asc, count, desc } from 'drizzle-orm';
import { drizzle, DrizzleSqliteDODatabase } from 'drizzle-orm/durable-sqlite';
import { migrate } from 'drizzle-orm/durable-sqlite/migrator';
import migrations from './db/drizzle/migrations';
import { logs } from './db/schema';

export interface Env {
  DO_LOG: DurableObjectNamespace<DoLog>;
  DO_LOG_PREFIX?: string;
}

export class DoLog extends DurableObject<Env> {
  storage: DurableObjectStorage;
  db: DrizzleSqliteDODatabase<any>;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    this.storage = ctx.storage;
    this.db = drizzle(this.storage, { logger: false });
    // Make sure all migrations complete before accepting queries.
    // Otherwise you will need to run `this.migrate()` in any function
    // that accesses the Drizzle database `this.db`.
    ctx.blockConcurrencyWhile(async () => {
      await this._migrate();
    });
  }
  private async _migrate() {
    migrate(this.db, migrations);
  }

  async write(message: unknown) {
    return this.db.insert(logs).values({ message }).returning({ id: logs.id });
  }

  async tail(limit = 100) {
    return this.db.select().from(logs).orderBy(desc(logs.id)).limit(limit);
  }

  async head(limit = 100) {
    return this.db.select().from(logs).orderBy(asc(logs.id)).limit(limit);
  }

  async count() {
    return (await this.db.select({ count: count() }).from(logs))[0].count;
  }
}

export class DoLogKV extends DurableObject<Env> {
  readonly prefix: string;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    this.prefix = env.DO_LOG_PREFIX ?? 'logs:';
  }

  async write(message: unknown) {
    const id = Date.now().toString();
    return this.ctx.storage.put(`${this.prefix}${id}`, message);
  }

  async tail(limit = 100) {
    return await this.ctx.storage.list({
      prefix: this.prefix,
      reverse: true,
      limit,
    });
  }

  async head(limit = 100) {
    return await this.ctx.storage.list({
      prefix: this.prefix,
      reverse: false,
      limit,
    });
  }

  async count() {
    let cursor: string | undefined = undefined;
    let total = 0;

    do {
      const list: Map<string, unknown> = await this.ctx.storage.list({
        prefix: this.prefix,
        reverse: false,
        limit: 1000,
        startAfter: cursor,
      });
      total += list.size;
      cursor = Array.from(list.keys()).at(-1);
    } while (cursor);

    return total;
  }
}
