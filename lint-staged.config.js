export default {
  "*.{js,ts,jsx,tsx,json,md,yaml,yml}": "prettier --write",
  "*.rs": () => ["cargo fmt", "cargo clippy -- -D warnings"],
};
