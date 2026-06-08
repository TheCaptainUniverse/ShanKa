export interface SettingsDraft {
  apiKey: string;
  baseUrl: string;
  model: string;
}

export const defaultSettingsDraft: SettingsDraft = {
  apiKey: "",
  baseUrl: "https://api.openai.com/v1",
  model: "",
};
