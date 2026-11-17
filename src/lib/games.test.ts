import { describe, expect, it } from "vitest";
import { gameOf, pathOf } from "./games";

describe("games", () => {
  it("treats the installed app's empty start path as Roblox Home", () => {
    expect(pathOf("")).toBe("/");
    expect(pathOf("/luduvo")).toBe("/luduvo");
    expect(gameOf("").id).toBe("roblox");
  });

  it("files each page under its game", () => {
    expect(gameOf("/luduvo").id).toBe("luduvo");
    expect(gameOf("/luduvo/users/3").id).toBe("luduvo");
    expect(gameOf("/users/3").id).toBe("roblox");
    expect(gameOf("/games/1").id).toBe("roblox");
    expect(gameOf("/luduvoish").id).toBe("roblox");
  });
});
