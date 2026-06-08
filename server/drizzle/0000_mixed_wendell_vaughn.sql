CREATE TABLE `configs` (
	`id` text PRIMARY KEY NOT NULL,
	`api_key` text,
	`base_url` text DEFAULT 'https://api.openai.com/v1' NOT NULL,
	`model` text DEFAULT '' NOT NULL,
	`timeout_ms` integer DEFAULT 5000 NOT NULL,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `hotkeys` (
	`id` text PRIMARY KEY NOT NULL,
	`mode` text NOT NULL,
	`accelerator` text NOT NULL,
	`enabled` integer DEFAULT true NOT NULL,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `personas` (
	`id` text PRIMARY KEY NOT NULL,
	`name` text NOT NULL,
	`system_prompt` text NOT NULL,
	`is_built_in` integer DEFAULT false NOT NULL,
	`is_active` integer DEFAULT false NOT NULL,
	`sort_order` integer DEFAULT 0 NOT NULL,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
