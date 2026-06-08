import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{vue,ts}", "./shared/**/*.ts"],
  theme: {
    extend: {
      colors: {
        shanka: {
          canvas: "#171717",
          panel: "#212121",
          input: "#2F2F2F",
          success: "#10A37F",
          border: "rgba(255,255,255,0.10)",
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "sans-serif",
        ],
      },
    },
  },
  plugins: [],
} satisfies Config;
