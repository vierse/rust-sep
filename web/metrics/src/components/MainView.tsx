import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend,
} from 'chart.js';
import { Bar } from 'react-chartjs-2';
import { TextField, Box, IconButton, Button } from "@radix-ui/themes";


import React from "react";

ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend
);

const labels = ["Monday", "Tuesday", "Wendsday", "Thursday", "Friday", "Saturday", "Sunday"];

export const data = {
  labels,
  datasets: [
    {
      label: 'Dataset 1',
      data: [0, 1, 2, 3, 4, 5, 6],
      backgroundColor: "red",
    },
    {
      label: 'Dataset 2',
      data: [6, 5, 4, 3, 2, 1, 0],
      backgroundColor: "green", 
    },
  ],
};

export function MainView() {
  return (
    <>
      <Bar
        data={data}
      />
    </>
  )
}
