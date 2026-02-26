import { Box, Flex, SegmentedControl } from "@radix-ui/themes";

import { Route, Switch } from "wouter";

import { MainView } from "./components/MainView";
import { UserView } from "./components/UserView";
import { NotifyProvider } from "./components/NotifyProvider";
import { UnlockView } from "./components/UnlockView";
import { CollectionView } from "./components/CollectionView";
import { CollectionCreator } from "./components/CollectionCreator";
import { Link1Icon, RowsIcon } from "@radix-ui/react-icons";

import React from "react";

type Mode = "single" | "collection";

export default function App() {
  const [mode, setMode] = React.useState<Mode>("single");

  return (
    <NotifyProvider>
      <Switch>
        <Route path="/unlock/:alias">{(params) => <UnlockView alias={params.alias} />}</Route>
        <Route path="/collection/:alias">{(params) => <CollectionView alias={params.alias} />}</Route>
        <Route>
          <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
            <Box style={{ position: "absolute", top: 16, right: 16, zIndex: 10 }}>
              <UserView />
            </Box>
            <Box style={{ position: "absolute", top: 16, left: 16, zIndex: 10 }}>
              <SegmentedControl.Root value={mode} onValueChange={(v) => setMode(v as Mode)}>
                <SegmentedControl.Item value="single"><Link1Icon style={{ transform: "translateY(3px)" }} /></SegmentedControl.Item>
                <SegmentedControl.Item value="collection"><RowsIcon style={{ transform: "translateY(3px)" }} /></SegmentedControl.Item>
              </SegmentedControl.Root>
            </Box>
            <Flex gap="2" align="center">
              {mode === "single" ? <MainView /> : <CollectionCreator />}
            </Flex>
          </Flex>
        </Route>
      </Switch>
    </NotifyProvider>
  );
}