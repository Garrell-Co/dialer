"use client";

import { cn } from "@/lib/utils";
import { signInWithEmailOtp, signInWithPassword } from "@/auth/auth-actions";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useActionState, useEffect } from "react";
import { useFormStatus } from "react-dom";

function SubmitButton() {
  const { pending } = useFormStatus();
  return (
    <Button type="submit" className="w-full" disabled={pending}>
      {pending ? "Logging in..." : "Login"}
    </Button>
  );
}

type SignInState =
  | { error: string }
  | { success: true; redirectTo: string }
  | null;

type EmailOtpState = { error?: string; success?: boolean } | null;

async function signInAction(
  prevState: SignInState,
  formData: FormData,
): Promise<SignInState> {
  const result = await signInWithPassword(formData);
  return result;
}

async function emailOtpAction(
  prevState: EmailOtpState,
  formData: FormData,
): Promise<EmailOtpState> {
  const result = await signInWithEmailOtp(formData);
  return result;
}

export function LoginForm({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
  const [state, formAction] = useActionState(signInAction, null);
  const [emailOtpState, emailOtpFormAction] = useActionState(
    emailOtpAction,
    null,
  );
  const router = useRouter();

  useEffect(() => {
    if (state && "success" in state && state.success && state.redirectTo) {
      router.push(state.redirectTo);
    }
  }, [state, router]);

  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader>
          <CardTitle className="text-2xl">Login</CardTitle>
          <CardDescription>
            Enter your email below to login with your password or request a
            one-time link.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form action={formAction}>
            <div className="flex flex-col gap-6">
              <div className="grid gap-2">
                <Label htmlFor="password-email">Email</Label>
                <Input
                  id="password-email"
                  name="email"
                  type="email"
                  placeholder="m@example.com"
                  required
                />
              </div>
              <div className="grid gap-2">
                <div className="flex items-center">
                  <Label htmlFor="password">Password</Label>
                  <Link
                    href="/auth/forgot-password"
                    className="ml-auto inline-block text-sm underline-offset-4 hover:underline"
                  >
                    Forgot your password?
                  </Link>
                </div>
                <Input
                  id="password"
                  name="password"
                  type="password"
                  required
                />
              </div>
              {state && "error" in state && (
                <p className="text-sm text-red-500">{state.error}</p>
              )}
              <SubmitButton />
            </div>
            <div className="mt-4 text-center text-sm">
              Don&apos;t have an account?{" "}
              <Link
                href="/auth/sign-up"
                className="underline underline-offset-4"
              >
                Sign up
              </Link>
            </div>
          </form>
          <div className="mt-6 border-t pt-4">
            <p className="mb-2 text-sm text-muted-foreground">
              Or receive a one-time login link to your email
            </p>
            {emailOtpState?.success ? (
              <p className="text-sm text-green-600">
                Check your email for a login link. You can close this window
                after you&apos;ve clicked the link.
              </p>
            ) : (
              <form action={emailOtpFormAction} className="space-y-4">
                <div className="grid gap-2">
                  <Label htmlFor="otp-email">Email</Label>
                  <Input
                    id="otp-email"
                    name="email"
                    type="email"
                    placeholder="m@example.com"
                    required
                  />
                </div>
                {emailOtpState?.error && (
                  <p className="text-sm text-red-500">
                    {emailOtpState.error}
                  </p>
                )}
                <Button
                  type="submit"
                  className="w-full"
                  variant="outline"
                  disabled={emailOtpState?.success}
                >
                  Send magic link
                </Button>
              </form>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
