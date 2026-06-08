export interface RuntimeConfig {
  apiKey: string;
  baseUrl: string;
  model: string;
  timeoutMs: number;
}

export async function getRuntimeConfig(): Promise<RuntimeConfig> {
  return {
    apiKey: Bun.env.OPENAI_API_KEY ?? "",
    baseUrl: Bun.env.OPENAI_BASE_URL ?? "https://api.openai.com/v1",
    model: Bun.env.OPENAI_MODEL ?? "",
    timeoutMs: Number(Bun.env.SHANKA_LLM_TIMEOUT_MS ?? 5000),
  };
}
