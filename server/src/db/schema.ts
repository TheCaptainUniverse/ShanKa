import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const configs = sqliteTable("configs", {
  id: text("id").primaryKey(),
  apiKey: text("api_key"),
  baseUrl: text("base_url").notNull().default("https://api.openai.com/v1"),
  model: text("model").notNull().default(""),
  timeoutMs: integer("timeout_ms").notNull().default(5000),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updated_at", { mode: "timestamp" }).notNull(),
});

export const personas = sqliteTable("personas", {
  id: text("id").primaryKey(),
  name: text("name").notNull(),
  systemPrompt: text("system_prompt").notNull(),
  isBuiltIn: integer("is_built_in", { mode: "boolean" }).notNull().default(false),
  isActive: integer("is_active", { mode: "boolean" }).notNull().default(false),
  sortOrder: integer("sort_order").notNull().default(0),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updated_at", { mode: "timestamp" }).notNull(),
});

export const hotkeys = sqliteTable("hotkeys", {
  id: text("id").primaryKey(),
  mode: text("mode", { enum: ["safe", "magic"] }).notNull(),
  accelerator: text("accelerator").notNull(),
  enabled: integer("enabled", { mode: "boolean" }).notNull().default(true),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updated_at", { mode: "timestamp" }).notNull(),
});
