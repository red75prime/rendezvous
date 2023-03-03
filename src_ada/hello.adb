with Ada.Text_IO;           use Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Calendar;          use Ada.Calendar;
with Ada.Assertions;        use Ada.Assertions;

procedure Hello is
   task Test is
      entry Noop;
      entry Ping (s : in Unbounded_String; r : out Unbounded_String);
   end Test;

   task body Test is
   begin
      loop
         select
            accept Noop do
               null;
            end Noop;
         or
            accept Ping (s : in Unbounded_String; r : out Unbounded_String) do
               r := s & To_Unbounded_String (" It's processed");
            end Ping;
         or
            terminate;
         end select;
      end loop;
   end Test;

   Start_Time   : Time;
   Elapsed_Time : Duration;
   Cycles       : constant Integer := 100_000;
   R            : Unbounded_String;
begin

   Start_Time := Clock;
   for i in 1 .. Cycles loop
      Test.Noop;
   end loop;
   Elapsed_Time := Clock - Start_Time;
   Put_Line
     ("one Test.Noop call in " &
      Duration'Image (Elapsed_Time / Cycles * 1_000_000) & "µs");
   Start_Time := Clock;
   for i in 1 .. Cycles loop
      Test.Ping (To_Unbounded_String ("aha"), R);
      Assert (R = To_Unbounded_String ("aha It's processed"));
   end loop;
   Elapsed_Time := Clock - Start_Time;
   Put_Line
     ("one Test.Ping call in " &
      Duration'Image (Elapsed_Time / Cycles * 1_000_000) & "µs");
   Put_Line (To_String (R));
end Hello;
