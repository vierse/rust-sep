import { Flex } from "@radix-ui/themes";

import { Route, Switch } from "wouter";

import { MainView } from "./components/MainView";
import { UserView } from "./components/UserView";
import { NotifyProvider } from "./components/NotifyProvider";
import { UnlockView } from "./components/UnlockView";

export default function App() {
  return (
    <NotifyProvider>
      <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
        <Switch>
          <Route path="/unlock/:alias">{(params) => <UnlockView alias={params.alias} />}</Route>
          <Route>
            <div style={{ position: "absolute", top: 16, right: 16, zIndex: 10 }}>
              <UserView />
            </div>
            <Flex gap="2" align="center">
              <MainView />
            </Flex>
          </Route>

        </Switch>
      </Flex>
    </NotifyProvider>
  );
}