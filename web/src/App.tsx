import { Flex } from "@radix-ui/themes";

import { Route, Switch } from "wouter";

import { MainView } from "./components/MainView";
import { UserView } from "./components/UserView";
import { NotifyProvider } from "./components/NotifyProvider";
import { UnlockView } from "./components/UnlockView";
import { CollectionView } from "./components/CollectionView";

export default function App() {
  return (
    <NotifyProvider>
      <Switch>
        <Route path="/unlock/:alias">{(params) => <UnlockView alias={params.alias} />}</Route>
        <Route path="/collection/:alias">{(params) => <CollectionView alias={params.alias} />}</Route>
        <Route>
          <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
            <div style={{ position: "absolute", top: 16, right: 16, zIndex: 10 }}>
              <UserView />
            </div>
            <Flex gap="2" align="center">
              <MainView />
            </Flex>
          </Flex>
        </Route>

      </Switch>
    </NotifyProvider>
  );
}