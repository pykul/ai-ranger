import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, within, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";

afterEach(cleanup);
import { generatePageNumbers, PaginationBar } from "../pages/Events";

// -- generatePageNumbers unit tests ------------------------------------------

describe("generatePageNumbers", () => {
  it("returns all pages when totalPages <= 7", () => {
    expect(generatePageNumbers(1, 5)).toEqual([1, 2, 3, 4, 5]);
    expect(generatePageNumbers(3, 7)).toEqual([1, 2, 3, 4, 5, 6, 7]);
    expect(generatePageNumbers(1, 1)).toEqual([1]);
  });

  it("includes ellipsis for large page counts, current page near start", () => {
    const pages = generatePageNumbers(2, 24);
    expect(pages[0]).toBe(1);
    expect(pages[pages.length - 1]).toBe(24);
    expect(pages).toContain(-1);
    expect(pages).toContain(2);
  });

  it("includes ellipsis for large page counts, current page near end", () => {
    const pages = generatePageNumbers(23, 24);
    expect(pages[0]).toBe(1);
    expect(pages[pages.length - 1]).toBe(24);
    expect(pages).toContain(-1);
    expect(pages).toContain(23);
  });

  it("includes two ellipsis when current page is in the middle", () => {
    const pages = generatePageNumbers(12, 24);
    expect(pages[0]).toBe(1);
    expect(pages[pages.length - 1]).toBe(24);
    const ellipsisCount = pages.filter((p) => p === -1).length;
    expect(ellipsisCount).toBe(2);
    expect(pages).toContain(11);
    expect(pages).toContain(12);
    expect(pages).toContain(13);
  });

  it("always includes first and last page", () => {
    for (const current of [1, 5, 10, 50, 100]) {
      const pages = generatePageNumbers(current, 100);
      expect(pages[0]).toBe(1);
      expect(pages[pages.length - 1]).toBe(100);
    }
  });

  it("does not exceed expected number of real page entries", () => {
    const pages = generatePageNumbers(50, 100);
    const realPages = pages.filter((p) => p !== -1);
    // Should have: 1, 49, 50, 51, 100 = 5 real pages
    expect(realPages.length).toBeLessThanOrEqual(7);
  });
});

// -- PaginationBar rendering tests -------------------------------------------

function renderPagination(props: Partial<React.ComponentProps<typeof PaginationBar>> = {}) {
  const defaults = {
    page: 1,
    totalPages: 10,
    total: 250,
    limit: 25,
    onPageChange: vi.fn(),
    onLimitChange: vi.fn(),
  };
  const merged = { ...defaults, ...props };
  const { container } = render(
    <MemoryRouter>
      <PaginationBar {...merged} />
    </MemoryRouter>
  );
  return { ...merged, container };
}

describe("PaginationBar", () => {
  it("renders page size selector with 10, 25, 50, 100 options", () => {
    renderPagination();
    const select = screen.getByRole("combobox");
    const options = within(select).getAllByRole("option").map((o) =>
      Number(o.getAttribute("value"))
    );
    expect(options).toEqual([10, 25, 50, 100]);
  });

  it("shows correct total count", () => {
    renderPagination({ total: 1234 });
    expect(screen.getByText(/1,234 events/)).toBeInTheDocument();
  });

  it("renders page buttons for small page count", () => {
    const { container } = renderPagination({ page: 1, totalPages: 5 });
    // Count buttons that are page numbers (not prev/next)
    const pageButtons = container.querySelectorAll("button.min-w-\\[2rem\\]");
    expect(pageButtons.length).toBe(5);
  });

  it("shows ellipsis for large page counts", () => {
    renderPagination({ page: 12, totalPages: 24 });
    const ellipses = screen.getAllByText("...");
    expect(ellipses.length).toBe(2);
  });

  it("disables Previous button on page 1", () => {
    renderPagination({ page: 1 });
    expect(screen.getByLabelText("Previous page")).toBeDisabled();
  });

  it("disables Next button on last page", () => {
    renderPagination({ page: 10, totalPages: 10 });
    expect(screen.getByLabelText("Next page")).toBeDisabled();
  });

  it("calls onPageChange when a page button is clicked", async () => {
    const { container, onPageChange } = renderPagination({ page: 1, totalPages: 5 });
    // Find the button with text "3"
    const pageButtons = container.querySelectorAll("button.min-w-\\[2rem\\]");
    const page3 = Array.from(pageButtons).find((b) => b.textContent === "3");
    expect(page3).toBeTruthy();
    await userEvent.click(page3!);
    expect(onPageChange).toHaveBeenCalledWith(3);
  });

  it("calls onLimitChange when page size is changed", async () => {
    const { onLimitChange } = renderPagination({ limit: 25 });
    const select = screen.getByRole("combobox");
    await userEvent.selectOptions(select, "50");
    expect(onLimitChange).toHaveBeenCalledWith(50);
  });

  it("does not render page buttons when totalPages is 1", () => {
    renderPagination({ totalPages: 1 });
    expect(screen.queryByLabelText("Previous page")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Next page")).not.toBeInTheDocument();
  });
});

// -- URL param initialization tests ------------------------------------------

describe("URL param integration", () => {
  it("PaginationBar select reflects limit from props (simulating URL init)", () => {
    renderPagination({ page: 3, limit: 50, totalPages: 10 });
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.value).toBe("50");
  });

  it("current page button is highlighted", () => {
    const { container } = renderPagination({ page: 3, limit: 25, totalPages: 10 });
    const pageButtons = container.querySelectorAll("button.min-w-\\[2rem\\]");
    const page3 = Array.from(pageButtons).find((b) => b.textContent === "3");
    expect(page3).toBeTruthy();
    expect(page3!.className).toContain("font-medium");
  });

  it("page size change calls onLimitChange", async () => {
    const { onLimitChange } = renderPagination({ page: 5, limit: 25, totalPages: 10 });
    const select = screen.getByRole("combobox");
    await userEvent.selectOptions(select, "100");
    expect(onLimitChange).toHaveBeenCalledWith(100);
  });
});
