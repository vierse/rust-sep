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

import { getReq } from "/src/api";
import { CategorySelecter, allOptions } from "./CategorySelecter";

ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend
);


const colors = ["red", "green", "blue", "cyan", "magenta", "pink", "crimson"]

export function MainView() {

  const [useData, setData] = React.useState({});
  const [catState, setCatState] = React.useState([null]);

  const get_data = async (cats) => {
    if (cats === null) return;
    if (cats.length === 0) return;
    const res = await getReq("/metrics/data",
                          null,
                          new URLSearchParams({weekdays: cats.join(',')}));
    setData(res);
  };

  const onSelectValues = (value, index) => {
    const clonedCatState = structuredClone(catState);

    clonedCatState[index] = value;
    setCatState(clonedCatState);
    if (clonedCatState) { get_data(clonedCatState[index].map((opt) => opt.value)) };
  };

 
  const weekdays = ["Monday", "Tuesday", "Wendsday", "Thursday", "Friday", "Saturday", "Sunday"];
  let chart_data = {
    labels: weekdays,
    datasets: [],
  };

  let col_idx = 0;
  for (const [label, data] of Object.entries(useData)) {
    col_idx += 1;
    chart_data.datasets.push({
      label, data, backgroundColor: colors[col_idx]
    })
  }

  return (
    <>
      {catState.map((selectCount, index) => {
        const options = getOptionsToRender(catState, allOptions);
        return (
            <CategorySelecter
              value={catState[index]}
              options={options}
              onSelect={(value) => onSelectValues(value, index)}
              key={index}
            />
        );
      })}
      <Bar data={chart_data} />
    </>
  )
}

const getOptionsToRender = (allSelectedOptions, allOptions) => {
  const filteredOptions = allSelectedOptions.flatMap((options) => options);

  const optionsToRender =
      filteredOptions.length > 0 ? allOptions.filter(
            (option) => !filteredOptions.some((selectOption) =>
                option && selectOption && option.value == selectOption.value
              )
            ): allOptions;
  return optionsToRender;
}
