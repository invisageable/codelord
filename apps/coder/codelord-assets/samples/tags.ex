defmodule Example do
  use ExUnit.Case
  @moduletag :module_tag_name

  describe "example describle" do
    @describetag :describe_tag_name
  
    @tag :test_tag_name
    test "example test" do
      assert true
    end
  end
end
