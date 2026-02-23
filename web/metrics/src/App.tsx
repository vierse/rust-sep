import { Flex } from "@radix-ui/themes";
import React from "react";

// import { Route, Switch } from "wouter";

import { MainView } from "./components/MainView";

export default function App() {
  
  return (
    <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
      <Flex gap="2" align="center">
          <MainView />
      </Flex>
    </Flex>
  );
}
