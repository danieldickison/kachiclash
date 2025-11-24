interface AuthProviderInfo {
  display_name: string;
  login_url: string;
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
const singleProvider = document.getElementById(
  "single-provider",
) as HTMLElement;
const multipleProviders = document.getElementById(
  "multiple-providers",
) as HTMLElement;
const singleProviderName = document.getElementById(
  "single-provider-name",
) as HTMLElement;
const providersList = document.getElementById("providers-list") as HTMLElement;

lookupForm.addEventListener("submit", async (e) => {
  e.preventDefault();

  const username = usernameInput.value.trim();
  if (!username) {
    return;
  }

  // Reset UI
  lookupError.style.display = "none";
  lookupResult.style.display = "none";
  singleProvider.style.display = "none";
  multipleProviders.style.display = "none";
  providersList.innerHTML = "";
  lookupButton.disabled = true;
  lookupButton.textContent = "Looking up...";

  try {
    const response = await fetch(
      `/login/lookup?username=${encodeURIComponent(username)}`,
    );

    if (!response.ok) {
      showError(
        "Username not found. Please check the spelling or create a new account.",
      );
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

    if (providers.length === 1) {
      handleSingleProvider(providers[0]);
    } else {
      handleMultipleProviders(providers);
    }
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

function handleSingleProvider(provider: AuthProviderInfo): void {
  if (!provider.login_url) {
    showError("Unable to determine login provider. Please try again.");
    resetButton();
    return;
  }

  singleProviderName.textContent = provider.display_name;
  singleProvider.style.display = "block";
  lookupResult.style.display = "block";
  // Redirect after a brief delay so user sees the message
  setTimeout(() => {
    window.location.href = provider.login_url;
  }, 500);
}

function handleMultipleProviders(providers: AuthProviderInfo[]): void {
  multipleProviders.style.display = "block";
  lookupResult.style.display = "block";

  providers.forEach((provider) => {
    if (provider.login_url) {
      const link = document.createElement("a");
      link.href = provider.login_url;
      link.textContent = `Login with ${provider.display_name}`;
      providersList.appendChild(link);
    }
  });

  resetButton();
}
