import { DurableObject } from 'cloudflare:workers';
import { asc, count, desc } from 'drizzle-orm';
import { drizzle, DrizzleSqliteDODatabase } from 'drizzle-orm/durable-sqlite';
import { migrate } from 'drizzle-orm/durable-sqlite/migrator';
import migrations from './db/drizzle/migrations';
import { logs } from './db/schema';

export interface Env {
  DO_LOG: DurableObjectNamespace<DoLog>;
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
