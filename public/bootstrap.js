import init from "/pkg/frontend.js";

init().catch((error) => {
  console.error("failed to initialize liquid custom elements", error);
});
