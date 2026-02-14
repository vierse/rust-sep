import { Flex } from "@radix-ui/themes";

import { MainView } from "./components/MainView";
import { PasswordPrompt } from "./components/PasswordPrompt";
import { UserView } from "./components/UserView";
import { NotifyProvider } from "./components/NotifyProvider";

export default function App() {
  const params = new URLSearchParams(window.location.search);
  const unlockAlias = params.get("unlock");

  return (
    <NotifyProvider>
      <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
        <div style={{ position: "absolute", top: 16, right: 16, zIndex: 10 }}>
          <UserView />
        </div>
        <Flex gap="2" align="center">
          {unlockAlias ? (
            <PasswordPrompt alias={unlockAlias} />
          ) : (
            <MainView />
          )}
        </Flex>
      </Flex>
    </NotifyProvider>
  );
}