interface AuthProviderInfo {
  provider_name: string;
}

interface LookupResponse {
  providers: AuthProviderInfo[];
}

const lookupForm = document.getElementById("lookup-form") as HTMLFormElement;
const usernameInput = document.getElementById(
  "username-input",
) as HTMLInputElement;
const lookupButton = lookupForm.querySelector(
  "button[type=submit]",
) as HTMLButtonElement;
const lookupError = document.getElementById("lookup-error") as HTMLElement;
const lookupResult = document.getElementById("lookup-result") as HTMLElement;
const multipleProviders = document.getElementById(
  "multiple-providers",
) as HTMLElement;
const discordLoginBtn = document.getElementById(
  "discord-login-btn",
) as HTMLAnchorElement;
const googleLoginBtn = document.getElementById(
  "google-login-btn",
) as HTMLAnchorElement;
const redditLoginBtn = document.getElementById(
  "reddit-login-btn",
) as HTMLAnchorElement;

lookupForm.addEventListener("submit", async (e) => {
  e.preventDefault();

  const username = usernameInput.value.trim();
  if (!username) {
    return;
  }

  // Reset UI
  lookupError.style.display = "none";
  lookupResult.style.display = "none";
  multipleProviders.style.display = "none";
  // Hide all provider buttons
  discordLoginBtn.style.display = "none";
  googleLoginBtn.style.display = "none";
  redditLoginBtn.style.display = "none";
  lookupButton.disabled = true;
  lookupButton.textContent = "Looking up...";

  try {
    const response = await fetch(
      `/login/lookup?username=${encodeURIComponent(username)}`,
    );

    if (!response.ok) {
      if (response.status === 404) {
        showError(
          "Username not found. Please check the spelling or create a new account.",
        );
      } else {
        showError(
          "An error occurred while looking up your account. Please try again.",
        );
      }
      resetButton();
      return;
    }

    const data = (await response.json()) as LookupResponse;
    const providers = data.providers || [];

    if (providers.length === 0) {
      showError("Unable to determine login provider. Please try again.");
      resetButton();
      return;
    }

    handleMultipleProviders(providers);
  } catch (error) {
    console.error("Lookup error:", error);
    showError(
      "An error occurred while looking up your account. Please try again.",
    );
    resetButton();
  }
});

function showError(message: string): void {
  lookupError.textContent = message;
  lookupError.style.display = "block";
}

function resetButton(): void {
  lookupButton.disabled = false;
  lookupButton.textContent = "Login";
}

function handleMultipleProviders(providers: AuthProviderInfo[]): void {
  multipleProviders.style.display = "block";
  lookupResult.style.display = "block";

  // Show the appropriate provider buttons based on what's available
  providers.forEach((provider) => {
    const name = provider.provider_name.toLowerCase();

    if (name === "discord") {
      discordLoginBtn.style.display = "flex";
    } else if (name === "google") {
      googleLoginBtn.style.display = "flex";
    } else if (name === "reddit") {
      redditLoginBtn.style.display = "flex";
    }
  });

  resetButton();
}
