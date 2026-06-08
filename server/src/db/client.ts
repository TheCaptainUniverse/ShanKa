import { Database } from "bun:sqlite";
import { drizzle } from "drizzle-orm/bun-sqlite";
import * as schema from "./schema";

const sqlite = new Database("server/data/shanka.db", { create: true });

export const db = drizzle(sqlite, { schema });
