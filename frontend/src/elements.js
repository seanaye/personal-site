function defineController(name, create) {
  if (customElements.get(name)) return;

  customElements.define(
    name,
    class extends HTMLElement {
      connectedCallback() {
        this.__liquidController ??= create(this);
        this.__liquidController.connected();
      }

      disconnectedCallback() {
        this.__liquidController?.disconnected();
      }
    },
  );
}

export function defineElements(createLiquid, createBackground, createPalette, createHue) {
  defineController("liquid-canvas", createLiquid);
  defineController("poline-background", createBackground);
  defineController("poline-palette", createPalette);
  // Register the slider last so its initial event reaches the other elements.
  defineController("hue-slider", createHue);
}
