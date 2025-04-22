import { adminTrigger } from "./push.js";

const pushAnnouncementForm = document.getElementById(
  "push-announcement",
) as HTMLFormElement;
pushAnnouncementForm.addEventListener("submit", (event) => {
  const messageInput = pushAnnouncementForm.elements.namedItem(
    "message",
  ) as HTMLInputElement;
  const delayInput = pushAnnouncementForm.elements.namedItem(
    "delay",
  ) as HTMLInputElement;
  const submitButton = pushAnnouncementForm.elements.namedItem(
    "submit",
  ) as HTMLButtonElement;
  submitButton.disabled = true;
  event.preventDefault();
  if (!messageInput.value) return;
  setTimeout(
    async () => {
      await adminTrigger({ Announcement: messageInput.value });
      submitButton.disabled = false;
    },
    1000 * parseFloat(delayInput.value),
  );
});
