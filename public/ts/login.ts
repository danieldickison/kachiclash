interface LookupResponse {
  providers: AuthProviderInfo[];
}

interface AuthProviderInfo {
  name: string;
  display_name: string;
}

const providerUrls: Record<string, string> = {
  discord: "/login/discord",
  google: "/login/google",
  reddit: "/login/reddit",
};

const lookupForm = document.getElementById("lookup-form") as HTMLFormElement;
const usernameInput = document.getElementById("username-input") as HTMLInputElement;
const lookupButton = lookupForm.querySelector("button") as HTMLButtonElement;
const lookupError = document.getElementById("lookup-error") as HTMLElement;
const lookupResult = document.getElementById("lookup-result") as HTMLElement;
const singleProvider = document.getElementById("single-provider") as HTMLElement;
const multipleProviders = document.getElementById("multiple-providers") as HTMLElement;
const singleProviderName = document.getElementById("single-provider-name") as HTMLElement;
const providersList = document.getElementById("provider-buttons") as HTMLElement;

lookupForm.addEventListener("submit", async (e: SubmitEvent) => {
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
    const response = await fetch(`/login/lookup?username=${encodeURIComponent(username)}`);

    if (!response.ok) {
      lookupError.textContent = "Username not found. Please check the spelling or create a new account.";
      lookupError.style.display = "block";
      lookupButton.disabled = false;
      lookupButton.textContent = "Login";
      return;
    }

    const data = (await response.json()) as LookupResponse;
    const providers = data.providers || [];

    if (providers.length === 0) {
      lookupError.textContent = "Unable to determine login provider. Please try again.";
      lookupError.style.display = "block";
      lookupButton.disabled = false;
      lookupButton.textContent = "Login";
      return;
    }

    if (providers.length === 1) {
      // Single provider: auto-redirect
      const provider = providers[0];
      const url = providerUrls[provider.name];
      if (url) {
        singleProviderName.textContent = provider.display_name;
        singleProvider.style.display = "block";
        lookupResult.style.display = "block";
        // Redirect after a brief delay so user sees the message
        setTimeout(() => {
          window.location.href = url;
        }, 500);
      } else {
        lookupError.textContent = "Unknown login provider. Please try again.";
        lookupError.style.display = "block";
        lookupButton.disabled = false;
        lookupButton.textContent = "Login";
      }
    } else {
      // Multiple providers: show list
      multipleProviders.style.display = "block";
      lookupResult.style.display = "block";

      providers.forEach((provider) => {
        const url = providerUrls[provider.name];
        if (url) {
          const link = document.createElement("a");
          link.href = url;
          link.className = `provider-login ${provider.name}-login`;
          link.textContent = `Login with ${provider.display_name}`;
          providersList.appendChild(link);
        }
      });

      lookupButton.disabled = false;
      lookupButton.textContent = "Login";
    }
  } catch (error) {
    console.error("Lookup error:", error);
    lookupError.textContent = "An error occurred while looking up your account. Please try again.";
    lookupError.style.display = "block";
    lookupButton.disabled = false;
    lookupButton.textContent = "Login";
  }
});
