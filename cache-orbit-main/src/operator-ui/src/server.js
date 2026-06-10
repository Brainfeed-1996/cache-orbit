const PORT = 8080;
const app = require("./app");

app
  .listen(PORT, () => {
    console.log(`operator-ui ready on http://localhost:${PORT}`);
  })
  .on("error", (err) => {
    console.error("fatal", err);
    process.exit(1);
  });
