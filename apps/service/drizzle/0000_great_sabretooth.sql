CREATE TABLE `keys` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`value` text,
	`name` text,
	`created_at` integer
);
--> statement-breakpoint
CREATE UNIQUE INDEX `keys_value_unique` ON `keys` (`value`);--> statement-breakpoint
CREATE TABLE `logs` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`message` text,
	`created_at` integer
);
