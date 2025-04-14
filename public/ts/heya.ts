export default {};

const expelForms: NodeListOf<HTMLFormElement> =
  document.body.querySelectorAll("form.expel");
for (const form of expelForms) {
  form.addEventListener("submit", (event) => {
    const { member, heya } = form.dataset;
    if (heya === undefined) throw new Error("missing heya or member name");

    const msg =
      member === undefined
        ? `Are you sure you want to leave ${heya}?`
        : `Are you sure you want to expel ${member} from ${heya}?`;
    if (!confirm(msg)) {
      event.preventDefault();
    }
  });
}
