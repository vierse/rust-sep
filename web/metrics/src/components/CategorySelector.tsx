import Select from "react-select";

export const allOptions = [
    { label: "redirect", value: 0 },
    { label: "shorten", value: 1 },
    { label: "recently added", value: 2 },
    { label: "authenticate session", value: 3 },
    { label: "authenticate user", value: 4 },
    { label: "total", value: 5 },
];

export const CategorySelector = ({ value, options, onSelect }) => {
  return (
    <Select
      value={value}
      options={options}
      isMulti
      onChange={onSelect}
    />
  );
};


