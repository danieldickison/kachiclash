import { sendTestNotification } from "./push.js";
import {
  pushPermissionState,
  pushSubscriptionState,
  subscribeToPushNotifications,
  type SubscriptionState,
} from "./service-client.js";

interface FormControls extends HTMLCollectionBase {
  // [item: string]: HTMLElement | RadioNodeList
  name: HTMLInputElement;
  notifications: HTMLFieldSetElement;
  "test-notification": HTMLButtonElement;
}

const form = document.getElementById("settings-form") as HTMLFormElement;
const messages = form.querySelector(".messages") as HTMLElement;
const saveButton = form.querySelector(".save-button") as HTMLButtonElement;
const nameField = (form.elements as unknown as FormControls).name;
const testNotificationButton = (form.elements as unknown as FormControls)[
  "test-notification"
];
const notifications = (form.elements as unknown as FormControls).notifications;
const typeCheckboxes: NodeListOf<HTMLInputElement> =
  notifications.querySelectorAll('input[type="checkbox"]');

let subscriptionState: SubscriptionState | null = null;
let edited = false;

notifications.classList.toggle("ios", /iPhone|iPad/.test(navigator.userAgent));

form.addEventListener("submit", (event) => {
  event.preventDefault();
  void save();
});
form.addEventListener("input", () => {
  showMessage(false, "");
  edited = true;
  refreshBusyState(false);
});

testNotificationButton.addEventListener("click", (event) => {
  event.preventDefault();
  void sendTestNotification();
});

async function refreshState(): Promise<void> {
  try {
    refreshBusyState(true);
    const permission = await pushPermissionState();
    notifications.dataset.permissionState = permission;
    subscriptionState =
      permission === "granted" ? await pushSubscriptionState() : null;
    for (const checkbox of typeCheckboxes) {
      checkbox.checked =
        subscriptionState?.opt_in.includes(checkbox.value) ?? false;
    }
  } catch (error) {
    console.error(error);
    showMessage(true, "Failed to refresh state");
  } finally {
    refreshBusyState(false);
  }
}

function refreshBusyState(busy: boolean): void {
  form.classList.toggle("busy", busy);
  saveButton.disabled = busy || !edited;
  notifications.disabled = busy;
  nameField.disabled = busy;
  testNotificationButton.disabled = busy || subscriptionState == null;
}

async function save(): Promise<void> {
  refreshBusyState(true);
  showMessage(false, "");
  try {
    let pushSubscription: PushSubscription | null = null;
    const optIn = getOptInTypes();
    if (optIn.length > 0) {
      pushSubscription = await subscribeToPushNotifications();
    }

    const body = {
      name: nameField.value,
      push_subscription: pushSubscription?.toJSON(),
      notification_opt_in: optIn,
    };
    const resp = await fetch("/settings", {
      method: "POST",
      body: JSON.stringify(body),
      headers: {
        "Content-Type": "application/json",
      },
      credentials: "same-origin",
    });
    if (resp.ok) {
      showMessage(false, "Settings have been saved.");
    } else {
      const body = await resp.text();
      throw new Error(body);
    }
  } catch (error: unknown) {
    showMessage(true, (error as object).toString());
  } finally {
    edited = false;
    await refreshState();
    refreshBusyState(false);
  }
}

function showMessage(isError: boolean, message: string): void {
  messages.style.display = message !== "" ? "block" : "none";
  messages.classList.toggle("error", isError);
  messages.innerText = message;
  messages.scrollIntoView();
}

function getOptInTypes(): string[] {
  const types: string[] = [];
  for (const checkbox of typeCheckboxes) {
    if (checkbox.checked) {
      types.push(checkbox.value);
    }
  }
  return types;
}

void refreshState();
