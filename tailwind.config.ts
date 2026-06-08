import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{vue,ts}", "./shared/**/*.ts"],
  theme: {
    extend: {
      colors: {
        shanka: {
          canvas: "rgb(var(--shanka-canvas) / <alpha-value>)",
          panel: "rgb(var(--shanka-panel) / <alpha-value>)",
          input: "rgb(var(--shanka-input) / <alpha-value>)",
          success: "rgb(var(--shanka-success) / <alpha-value>)",
          danger: "rgb(var(--shanka-danger) / <alpha-value>)",
          primary: "rgb(var(--shanka-primary) / <alpha-value>)",
          secondary: "rgb(var(--shanka-secondary) / <alpha-value>)",
          muted: "rgb(var(--shanka-muted) / <alpha-value>)",
          hover: "rgb(var(--shanka-hover) / <alpha-value>)",
          focus: "rgb(var(--shanka-focus) / <alpha-value>)",
          border: "var(--shanka-border)",
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
