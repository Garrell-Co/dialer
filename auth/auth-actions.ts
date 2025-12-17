"use server";

import { createClient } from "@/lib/supabase/server";
import { getURL } from "@/lib/utils";
import { revalidatePath } from "next/cache";

export async function signUp(formData: FormData) {
  const email = formData.get("email") as string;
  const password = formData.get("password") as string;
  const repeatPassword = formData.get("repeatPassword") as string;

  console.log("[signUp] Attempting sign up for email:", email);

  if (!email || !password) {
    console.log("[signUp] Validation failed: missing email or password");
    return { error: "Email and password are required" };
  }

  if (password !== repeatPassword) {
    console.log("[signUp] Validation failed: passwords do not match");
    return { error: "Passwords do not match" };
  }

  const supabase = await createClient();
  const redirectUrl = `${getURL()}protected`;
  console.log("[signUp] Redirect URL:", redirectUrl);

  const { error } = await supabase.auth.signUp({
    email,
    password,
    options: {
      emailRedirectTo: redirectUrl,
    },
  });

  if (error) {
    console.error("[signUp] Error:", error.message);
    return { error: error.message };
  }

  console.log("[signUp] Success: user signed up");
  revalidatePath("/", "layout");
  return { success: true as const, redirectTo: "/auth/sign-up-success" };
}

export async function signInWithPassword(formData: FormData) {
  const email = formData.get("email") as string;
  const password = formData.get("password") as string;

  console.log("[signInWithPassword] Attempting sign in for email:", email);

  if (!email || !password) {
    console.log("[signInWithPassword] Validation failed: missing email or password");
    return { error: "Email and password are required" };
  }

  const supabase = await createClient();

  const { error } = await supabase.auth.signInWithPassword({
    email,
    password,
  });

  if (error) {
    console.error("[signInWithPassword] Error:", error.message);
    return { error: error.message };
  }

  console.log("[signInWithPassword] Success: user signed in");
  revalidatePath("/", "layout");
  return { success: true as const, redirectTo: "/protected" };
}

export async function resetPasswordForEmail(formData: FormData) {
  const email = formData.get("email") as string;

  console.log("[resetPasswordForEmail] Requesting password reset for email:", email);

  if (!email) {
    console.log("[resetPasswordForEmail] Validation failed: email is required");
    return { error: "Email is required" };
  }

  const supabase = await createClient();
  const redirectUrl = `${getURL()}auth/update-password`;
  console.log("[resetPasswordForEmail] Redirect URL:", redirectUrl);

  const { error } = await supabase.auth.resetPasswordForEmail(email, {
    redirectTo: redirectUrl,
  });

  if (error) {
    console.error("[resetPasswordForEmail] Error:", error.message);
    return { error: error.message };
  }

  console.log("[resetPasswordForEmail] Success: password reset email sent");
  return { success: true };
}

export async function updateUserPassword(formData: FormData) {
  const password = formData.get("password") as string;

  console.log("[updateUserPassword] Attempting to update password");

  if (!password) {
    console.log("[updateUserPassword] Validation failed: password is required");
    return { error: "Password is required" };
  }

  const supabase = await createClient();

  const { error } = await supabase.auth.updateUser({ password });

  if (error) {
    console.error("[updateUserPassword] Error:", error.message);
    return { error: error.message };
  }

  console.log("[updateUserPassword] Success: password updated");
  revalidatePath("/", "layout");
  return { success: true as const, redirectTo: "/protected" };
}

