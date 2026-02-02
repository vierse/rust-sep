import { Flex } from "@radix-ui/themes";

import { useAppController } from "./controller";
import { MainView } from "./components/MainView";
import { UserView } from "./components/UserView";


export default function App() {
    const { state, dispatch } = useAppController();

    return (
        <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
            <div style={{ position: "absolute", top: 16, right: 16, zIndex: 10 }}>
                <UserView state={state} dispatch={dispatch} />
            </div>
            <Flex gap="2" align="center">
                <MainView state={state} dispatch={dispatch} />
            </Flex>
        </Flex >
    );
}