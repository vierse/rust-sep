import { Box, Card, Flex, Link, Text } from "@radix-ui/themes";
import { Link2Icon } from "@radix-ui/react-icons";

import React from "react";
import { getJson } from "../api";

type CollectionItem = {
  url: string;
  position: number;
};

export function CollectionView({ alias }: { alias: string }) {
  const [items, setItems] = React.useState<CollectionItem[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState("");

  const hash = window.location.hash;
  const highlighted = hash ? Number(hash.slice(1)) : null;

  React.useEffect(() => {
    (async () => {
      try {
        const data = await getJson<CollectionItem[]>(
          `/api/collection/${encodeURIComponent(alias)}`
        );
        setItems(data);
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to load collection";
        setError(msg);
      } finally {
        setLoading(false);
      }
    })();
  }, [alias]);

  if (loading) {
    return <Text size="4">Loading...</Text>;
  }

  if (error) {
    return <Text size="4" color="red">{error}</Text>;
  }

  return (
    <Flex direction="column" gap="4" align="center">
      <Text size="3" weight="bold">{alias}</Text>

      <Flex direction="column" gap="2" style={{ width: "40rem", maxWidth: "90vw" }}>
        {items.map((item) => {
          const isHighlighted = highlighted === item.position;
          return (
            <Card
              key={item.position}
              style={{
                outline: isHighlighted ? "2px solid var(--accent-9)" : undefined,
              }}
            >
              <Flex align="center" gap="3">
                <Box>
                  <Text size="1" color="gray">#{item.position}</Text>
                </Box>
                <Link2Icon />
                <Link href={item.url} target="_blank" rel="noopener noreferrer" size="2">
                  {item.url}
                </Link>
              </Flex>
            </Card>
          );
        })}
      </Flex>
    </Flex>
  );
}
