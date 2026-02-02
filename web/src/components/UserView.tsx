import { Button } from "@radix-ui/themes";
import { PersonIcon } from "@radix-ui/react-icons";
import type { Dispatch } from "../controller";
import type { AppState } from "../model";


export function UserView({ state, dispatch }: { state: AppState, dispatch: Dispatch }) {
    return (
        <>
            <Button>
                <PersonIcon />Account
            </Button>
        </>
    );
}