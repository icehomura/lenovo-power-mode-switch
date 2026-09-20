import { createApp } from "vue";
import App from "./App.vue";
import Floater from "./Floater.vue";
import "./styles.css";

const params = new URLSearchParams(window.location.search);

if (params.get("window") === "floater") {
  createApp(Floater).mount("#app");
} else {
  createApp(App).mount("#app");
}
