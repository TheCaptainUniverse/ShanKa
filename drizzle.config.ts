import { defineConfig } from "drizzle-kit";

export default defineConfig({
  schema: "./server/src/db/schema.ts",
  out: "./server/drizzle",
  dialect: "sqlite",
  dbCredentials: {
    url: "./server/data/shanka.db",
  },
});
