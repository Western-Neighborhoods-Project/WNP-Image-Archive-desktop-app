import Root from "./dropdown-menu.svelte";
import Content from "./dropdown-menu-content.svelte";
import Item from "./dropdown-menu-item.svelte";
import Separator from "./dropdown-menu-separator.svelte";

export {
	Root,
	Content,
	Item,
	Separator,
	//
	Root as DropdownMenu,
	Content as DropdownMenuContent,
	Item as DropdownMenuItem,
	Separator as DropdownMenuSeparator,
};

export { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
