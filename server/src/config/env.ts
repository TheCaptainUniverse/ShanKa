export const serverEnv = {
  port: Number(Bun.env.SHANKA_SERVER_PORT ?? 4317),
  databaseUrl: Bun.env.SHANKA_DATABASE_URL ?? "server/data/shanka.db",
};
